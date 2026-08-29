#![cfg(target_os = "windows")]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::undocumented_unsafe_blocks
    )
)]

use std::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE, E_NOTIMPL, E_POINTER,
    HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl, IServiceProvider,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Ole::{IObjectWithSite, IObjectWithSite_Impl};
use windows::Win32::UI::Shell::{
    ECS_DISABLED, ECS_ENABLED, ECS_HIDDEN, IEnumExplorerCommand, IExplorerCommand,
    IExplorerCommand_Impl, IFolderView, IShellBrowser, IShellItem, IShellItemArray,
    SID_STopLevelBrowser, SIGDN_FILESYSPATH,
};
use windows::core::{
    BOOL, GUID, HRESULT, IUnknown, Interface, PCWSTR, PWSTR, Ref, Result, implement,
};
use zifile_core::detect_format;

pub const CREATE_COMMAND_CLSID: GUID = GUID::from_u128(0x2f86f25d_3b76_4cd2_8fe8_9d7a2eefb531);
pub const EXTRACT_COMMAND_CLSID: GUID = GUID::from_u128(0x2d39ad2e_1b36_4f4f_8e09_589f0b1d2bc3);
const E_PENDING: HRESULT = HRESULT(0x8000_000A_u32 as i32);
const MAX_SELECTED_ITEMS: u32 = 256;
const MAX_ARGUMENT_UTF16_UNITS: usize = 24_000;
const WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x400;
static LIVE_OBJECTS: AtomicU32 = AtomicU32::new(0);
static SERVER_LOCKS: AtomicU32 = AtomicU32::new(0);

struct ModuleLifetime;

impl ModuleLifetime {
    fn acquire() -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::AcqRel);
        Self
    }
}

impl Drop for ModuleLifetime {
    fn drop(&mut self) {
        let _ = LIVE_OBJECTS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            count.checked_sub(1)
        });
    }
}

fn update_server_locks(lock: BOOL) {
    if lock.as_bool() {
        SERVER_LOCKS.fetch_add(1, Ordering::AcqRel);
    } else {
        let _ = SERVER_LOCKS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            count.checked_sub(1)
        });
    }
}

fn unload_result(live_objects: u32, server_locks: u32) -> HRESULT {
    if live_objects == 0 && server_locks == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

struct ExplorerSite {
    site: std::sync::Mutex<Option<IUnknown>>,
}

impl ExplorerSite {
    fn new() -> Self {
        Self {
            site: std::sync::Mutex::new(None),
        }
    }

    fn set(&self, site: Ref<IUnknown>) -> Result<()> {
        let mut current = self.site.lock().map_err(|_| E_FAIL)?;
        *current = site.as_ref().cloned();
        Ok(())
    }

    fn get(&self, interface_id: *const GUID, site: *mut *mut c_void) -> Result<()> {
        if interface_id.is_null() || site.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: `site` was checked for null and is the caller's writable output slot.
        unsafe { site.write(std::ptr::null_mut()) };
        let current = self
            .site
            .lock()
            .map_err(|_| E_FAIL)?
            .clone()
            .ok_or(E_NOINTERFACE)?;
        // SAFETY: the stored site is a live COM interface and `site` is a valid output slot.
        unsafe { current.query(interface_id, site).ok() }
    }

    fn current_folder_path(&self) -> Result<PathBuf> {
        let site = self
            .site
            .lock()
            .map_err(|_| E_FAIL)?
            .clone()
            .ok_or(E_FAIL)?;
        let service_provider: IServiceProvider = site.cast()?;
        // SAFETY: the service provider is a live Explorer site and the requested
        // service/interface pair is defined by the Shell browser contract.
        let browser: IShellBrowser =
            unsafe { service_provider.QueryService(&SID_STopLevelBrowser)? };
        // SAFETY: Explorer owns the active view returned by the browser service.
        let view = unsafe { browser.QueryActiveShellView()? };
        let folder_view: IFolderView = view.cast()?;
        // SAFETY: the folder view is a live Shell interface supplied by Explorer.
        let folder: IShellItem = unsafe { folder_view.GetFolder()? };
        // SAFETY: a successful call returns a COM-task-allocated display name.
        let display_name = unsafe { folder.GetDisplayName(SIGDN_FILESYSPATH)? };
        // SAFETY: the returned buffer is a valid NUL-terminated UTF-16 string.
        let path_result = unsafe { display_name.to_string() }
            .map(PathBuf::from)
            .map_err(|_| windows::core::Error::from(E_FAIL));
        // SAFETY: the Shell allocates this string with the COM task allocator.
        unsafe { CoTaskMemFree(Some(display_name.0.cast())) };
        path_result
    }
}

#[implement(IExplorerCommand, IObjectWithSite)]
struct ZiFileCreateCommand {
    _lifetime: ModuleLifetime,
    site: ExplorerSite,
}

impl IObjectWithSite_Impl for ZiFileCreateCommand_Impl {
    fn SetSite(&self, site: Ref<IUnknown>) -> Result<()> {
        self.site.set(site)
    }

    fn GetSite(&self, interface_id: *const GUID, site: *mut *mut c_void) -> Result<()> {
        self.site.get(interface_id, site)
    }
}

impl IExplorerCommand_Impl for ZiFileCreateCommand_Impl {
    fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        allocate_shell_string(if user_locale_is_chinese() {
            "使用 ZiFile 创建压缩文件"
        } else {
            "Create archive with ZiFile"
        })
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        shell_icon_resource()
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        allocate_shell_string(if user_locale_is_chinese() {
            "把选中的文件和文件夹发送到 ZiFile 创建页"
        } else {
            "Send the selected files and folders to the ZiFile create page"
        })
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(CREATE_COMMAND_CLSID)
    }

    fn GetState(&self, items: Ref<IShellItemArray>, ok_to_be_slow: BOOL) -> Result<u32> {
        if !ok_to_be_slow.as_bool() {
            return Err(E_PENDING.into());
        }
        let enabled = self.create_sources(items).is_ok();
        Ok(if enabled {
            ECS_ENABLED.0
        } else {
            ECS_DISABLED.0
        } as u32)
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _bind_context: Ref<IBindCtx>) -> Result<()> {
        let paths = self.create_sources(items)?;
        if paths.is_empty() {
            return Err(E_FAIL.into());
        }
        let desktop = sibling_desktop_path()?;
        Command::new(desktop)
            .arg("--create")
            .args(paths)
            .spawn()
            .map(|_| ())
            .map_err(|_| E_FAIL.into())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(0)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        Err(E_NOTIMPL.into())
    }
}

impl ZiFileCreateCommand_Impl {
    fn create_sources(&self, items: Ref<IShellItemArray>) -> Result<Vec<PathBuf>> {
        let Some(items) = items.as_ref() else {
            return self
                .site
                .current_folder_path()
                .map(|path| validate_create_paths(vec![path]))?;
        };
        // SAFETY: `items` is a live COM interface borrowed for this call.
        let count = unsafe { items.GetCount()? };
        if count == 0 {
            return self
                .site
                .current_folder_path()
                .map(|path| validate_create_paths(vec![path]))?;
        }
        if !create_selection_enabled(count) {
            return Err(E_FAIL.into());
        }
        validate_create_paths(collect_paths(items)?)
    }
}

#[implement(IExplorerCommand)]
struct ZiFileExtractCommand {
    _lifetime: ModuleLifetime,
}

impl IExplorerCommand_Impl for ZiFileExtractCommand_Impl {
    fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        allocate_shell_string(if user_locale_is_chinese() {
            "使用 ZiFile 解压到同名目录"
        } else {
            "Extract to matching folder with ZiFile"
        })
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        shell_icon_resource()
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        allocate_shell_string(if user_locale_is_chinese() {
            "列出压缩文件并解压到旁边的同名文件夹"
        } else {
            "List the archive and extract it to a matching sibling folder"
        })
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(EXTRACT_COMMAND_CLSID)
    }

    fn GetState(&self, items: Ref<IShellItemArray>, ok_to_be_slow: BOOL) -> Result<u32> {
        if !ok_to_be_slow.as_bool() {
            return Err(E_PENDING.into());
        }
        let supported = items
            .as_ref()
            .and_then(|items| single_path(items, "--extract-here").ok())
            .is_some_and(|path| extract_path_supported(&path));
        Ok(if supported {
            ECS_ENABLED.0
        } else {
            ECS_HIDDEN.0
        } as u32)
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _bind_context: Ref<IBindCtx>) -> Result<()> {
        let items = items
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        let path = single_path(items, "--extract-here")?;
        if !extract_path_supported(&path) {
            return Err(E_FAIL.into());
        }
        let desktop = sibling_desktop_path()?;
        Command::new(desktop)
            .arg("--extract-here")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|_| E_FAIL.into())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(0)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        Err(E_NOTIMPL.into())
    }
}

#[implement(IClassFactory)]
struct ZiFileCreateClassFactory {
    _lifetime: ModuleLifetime,
}

impl IClassFactory_Impl for ZiFileCreateClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if outer.as_ref().is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if iid.is_null() || object.is_null() {
            return Err(E_POINTER.into());
        }
        let unknown: IUnknown = ZiFileCreateCommand {
            _lifetime: ModuleLifetime::acquire(),
            site: ExplorerSite::new(),
        }
        .into();
        // SAFETY: both output pointers were checked above and `unknown` owns a live COM object.
        unsafe { unknown.query(iid, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        update_server_locks(_lock);
        Ok(())
    }
}

#[implement(IClassFactory)]
struct ZiFileExtractClassFactory {
    _lifetime: ModuleLifetime,
}

impl IClassFactory_Impl for ZiFileExtractClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if outer.as_ref().is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if iid.is_null() || object.is_null() {
            return Err(E_POINTER.into());
        }
        let unknown: IUnknown = ZiFileExtractCommand {
            _lifetime: ModuleLifetime::acquire(),
        }
        .into();
        // SAFETY: both output pointers were checked above and `unknown` owns a live COM object.
        unsafe { unknown.query(iid, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        update_server_locks(_lock);
        Ok(())
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(_instance: HINSTANCE, _reason: u32, _reserved: *mut c_void) -> BOOL {
    BOOL(1)
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    unload_result(
        LIVE_OBJECTS.load(Ordering::Acquire),
        SERVER_LOCKS.load(Ordering::Acquire),
    )
}

#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    ffi_hresult(|| dll_get_class_object(class_id, interface_id, object))
}

fn ffi_hresult(action: impl FnOnce() -> HRESULT) -> HRESULT {
    catch_unwind(AssertUnwindSafe(action)).unwrap_or(E_FAIL)
}

fn dll_get_class_object(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if object.is_null() {
        return E_POINTER;
    }
    // SAFETY: the caller supplied a non-null writable COM output slot.
    unsafe { object.write(std::ptr::null_mut()) };
    if class_id.is_null() || interface_id.is_null() {
        return E_POINTER;
    }
    // SAFETY: `class_id` was checked for null and COM requires it to point to a GUID.
    let factory: IClassFactory = match unsafe { *class_id } {
        CREATE_COMMAND_CLSID => ZiFileCreateClassFactory {
            _lifetime: ModuleLifetime::acquire(),
        }
        .into(),
        EXTRACT_COMMAND_CLSID => ZiFileExtractClassFactory {
            _lifetime: ModuleLifetime::acquire(),
        }
        .into(),
        _ => return CLASS_E_CLASSNOTAVAILABLE,
    };
    // SAFETY: `interface_id` and `object` were validated and `factory` is a live COM object.
    unsafe { factory.query(interface_id, object) }
}

fn collect_paths(items: &IShellItemArray) -> Result<Vec<PathBuf>> {
    // SAFETY: `items` is a live COM interface borrowed by the command invocation.
    let count = unsafe { items.GetCount()? };
    if count == 0 || count > MAX_SELECTED_ITEMS {
        return Err(E_FAIL.into());
    }
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        // SAFETY: the index is strictly below the count returned by this same COM array.
        let item = unsafe { items.GetItemAt(index)? };
        // SAFETY: `item` is live and requests a COM-allocated filesystem-path string.
        let display_name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        // SAFETY: a successful `GetDisplayName` returns a valid NUL-terminated UTF-16 string.
        let path_result = unsafe { display_name.to_string() }.map(PathBuf::from);
        // SAFETY: `display_name` came from the COM task allocator and is freed exactly once here.
        unsafe { CoTaskMemFree(Some(display_name.0.cast())) };
        let path = path_result?;
        paths.push(path);
    }
    Ok(paths)
}

fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if !unique
            .iter()
            .any(|existing| paths_have_same_identity(existing, &path))
        {
            unique.push(path);
        }
    }
    unique
}

fn validate_create_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let paths = deduplicate_paths(paths);
    if paths.is_empty()
        || paths.iter().any(|path| !is_real_file_or_directory(path))
        || !command_line_within_limit("--create", &paths)
    {
        return Err(E_FAIL.into());
    }
    Ok(paths)
}

fn paths_have_same_identity(left: &std::path::Path, right: &std::path::Path) -> bool {
    let normalize = |path: &std::path::Path| {
        let mut value = path.to_string_lossy().replace('/', "\\").to_lowercase();
        while value.len() > 3 && value.ends_with('\\') {
            value.pop();
        }
        value
    };
    normalize(left) == normalize(right)
}

fn single_path(items: &IShellItemArray, command_flag: &str) -> Result<PathBuf> {
    let paths = collect_paths(items)?;
    if !command_line_within_limit(command_flag, &paths) {
        return Err(E_FAIL.into());
    }
    if paths.len() != 1 {
        return Err(E_FAIL.into());
    }
    paths.into_iter().next().ok_or_else(|| E_FAIL.into())
}

fn extract_path_supported(path: &std::path::Path) -> bool {
    is_real_file(path)
        && detect_format(path).is_ok_and(|format| {
            let capabilities = format.capabilities();
            capabilities.list && capabilities.extract
        })
}

fn is_real_file_or_directory(path: &std::path::Path) -> bool {
    (path.is_file() || path.is_dir()) && !is_link_like(path)
}

fn is_real_file(path: &std::path::Path) -> bool {
    path.is_file() && !is_link_like(path)
}

fn is_link_like(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        if metadata.file_type().is_symlink() {
            return true;
        }
        use std::os::windows::fs::MetadataExt;
        u64::from(metadata.file_attributes()) & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0
    })
}

fn create_selection_enabled(count: u32) -> bool {
    (1..=MAX_SELECTED_ITEMS).contains(&count)
}

fn command_line_within_limit(command_flag: &str, paths: &[PathBuf]) -> bool {
    let units = paths
        .iter()
        .fold(command_flag.encode_utf16().count(), |total, path| {
            total
                .saturating_add(path.as_os_str().encode_wide().count())
                .saturating_add(3)
        });
    units <= MAX_ARGUMENT_UTF16_UNITS
}

fn sibling_desktop_path() -> Result<PathBuf> {
    let mut module = HMODULE::default();
    // SAFETY: the function address belongs to this loaded DLL and `module` is a writable output.
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(DllGetClassObject as *const () as *const u16),
            &mut module,
        )?;
    }
    let mut buffer = vec![0_u16; 32_768];
    // SAFETY: `module` was resolved above and `buffer` provides writable storage for the path.
    let length = unsafe { GetModuleFileNameW(Some(module), &mut buffer) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(E_FAIL.into());
    }
    let module_path = buffer.get(..length).ok_or(E_FAIL)?;
    let mut path = PathBuf::from(String::from_utf16_lossy(module_path));
    path.set_file_name("zifile-desktop.exe");
    Ok(path)
}

fn shell_icon_resource() -> Result<PWSTR> {
    let desktop = sibling_desktop_path()?;
    allocate_shell_string(&icon_resource_string(&desktop))
}

fn icon_resource_string(desktop: &std::path::Path) -> String {
    format!("{},0", desktop.display())
}

fn user_locale_is_chinese() -> bool {
    let mut locale = [0_u16; 85];
    // SAFETY: `locale` is a writable buffer sized to Windows' maximum locale-name length.
    let length = unsafe { GetUserDefaultLocaleName(&mut locale) };
    let Some(end) = usize::try_from(length)
        .ok()
        .filter(|length| *length > 2)
        .and_then(|length| length.checked_sub(1))
    else {
        return false;
    };
    locale
        .get(..end)
        .is_some_and(|name| String::from_utf16_lossy(name).starts_with("zh"))
}

fn allocate_shell_string(value: &str) -> Result<PWSTR> {
    let mut wide = value.encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    let bytes = wide.len() * size_of::<u16>();
    // SAFETY: the requested byte count exactly represents the UTF-16 buffer including its NUL.
    let destination = unsafe { CoTaskMemAlloc(bytes) }.cast::<u16>();
    if destination.is_null() {
        return Err(E_FAIL.into());
    }
    // SAFETY: allocation succeeded for `wide.len()` u16 values and the regions do not overlap.
    unsafe { destination.copy_from_nonoverlapping(wide.as_ptr(), wide.len()) };
    Ok(PWSTR(destination))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_clsid_is_stable() {
        assert_eq!(
            CREATE_COMMAND_CLSID,
            GUID::from_u128(0x2f86f25d_3b76_4cd2_8fe8_9d7a2eefb531)
        );
        assert_eq!(
            EXTRACT_COMMAND_CLSID,
            GUID::from_u128(0x2d39ad2e_1b36_4f4f_8e09_589f0b1d2bc3)
        );
    }

    #[test]
    fn ffi_hresult_contains_unexpected_panics() {
        assert_eq!(ffi_hresult(|| panic!("intentional FFI test panic")), E_FAIL);
    }

    #[test]
    fn dll_unload_requires_no_live_objects_or_server_locks() {
        assert_eq!(unload_result(0, 0), S_OK);
        assert_eq!(unload_result(1, 0), S_FALSE);
        assert_eq!(unload_result(0, 1), S_FALSE);
        assert_eq!(unload_result(u32::MAX, u32::MAX), S_FALSE);
    }

    #[test]
    fn dll_factory_rejects_invalid_inputs_and_clears_output() {
        let mut object = std::ptr::dangling_mut::<c_void>();
        assert_eq!(
            DllGetClassObject(std::ptr::null(), &IClassFactory::IID, &mut object),
            E_POINTER
        );
        assert!(object.is_null());

        object = std::ptr::dangling_mut::<c_void>();
        let unknown = GUID::from_u128(0x4a26c80b_16af_4247_b0a8_86d2d31fbfaa);
        assert_eq!(
            DllGetClassObject(&unknown, &IClassFactory::IID, &mut object),
            CLASS_E_CLASSNOTAVAILABLE
        );
        assert!(object.is_null());
    }

    #[test]
    fn command_line_budget_accepts_unicode_and_rejects_oversized_selection() {
        assert!(command_line_within_limit(
            "--create",
            &[
                PathBuf::from(r"C:\资料\甲.txt"),
                PathBuf::from(r"C:\资料\乙 folder"),
            ]
        ));
        assert!(!command_line_within_limit(
            "--extract-here",
            &[PathBuf::from("x".repeat(MAX_ARGUMENT_UTF16_UNITS))]
        ));
    }

    #[test]
    fn create_sources_deduplicate_windows_path_spellings() {
        let unique = deduplicate_paths(vec![
            PathBuf::from(r"C:\Data\Alpha.txt"),
            PathBuf::from(r"c:/data/alpha.txt"),
            PathBuf::from(r"C:\Data\Beta.txt"),
        ]);
        assert_eq!(
            unique,
            vec![
                PathBuf::from(r"C:\Data\Alpha.txt"),
                PathBuf::from(r"C:\Data\Beta.txt"),
            ]
        );
    }

    #[test]
    fn create_sources_deduplicate_unicode_and_root_path_spellings() {
        let unique = deduplicate_paths(vec![
            PathBuf::from(r"C:\资料\Ä.txt"),
            PathBuf::from(r"c:/资料/ä.txt"),
            PathBuf::from(r"C:\"),
            PathBuf::from(r"c:/"),
        ]);
        assert_eq!(
            unique,
            vec![PathBuf::from(r"C:\资料\Ä.txt"), PathBuf::from(r"C:\")]
        );
    }

    #[test]
    fn background_create_path_obeys_the_same_command_line_budget() {
        let oversized = PathBuf::from("x".repeat(MAX_ARGUMENT_UTF16_UNITS));
        let error = validate_create_paths(vec![oversized])
            .expect_err("an oversized Explorer background path must be rejected");
        assert_eq!(error.code(), E_FAIL);
    }

    #[test]
    fn create_path_validation_rejects_empty_sources() {
        let error = validate_create_paths(Vec::new())
            .expect_err("an empty Explorer source set must not launch the desktop");
        assert_eq!(error.code(), E_FAIL);
    }

    #[test]
    fn create_path_validation_rejects_missing_sources() {
        let error = validate_create_paths(vec![PathBuf::from(
            r"C:\this-path-does-not-exist\source.txt",
        )])
        .expect_err("a disappeared Explorer source must not launch the desktop");
        assert_eq!(error.code(), E_FAIL);
    }

    #[test]
    fn create_path_validation_accepts_real_files_and_directories() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let file = temporary.path().join("source.txt");
        let directory = temporary.path().join("folder");
        std::fs::write(&file, b"source").expect("source file");
        std::fs::create_dir(&directory).expect("source directory");

        let paths = validate_create_paths(vec![file.clone(), directory.clone()])
            .expect("real Explorer paths should be accepted");
        assert_eq!(paths, vec![file, directory]);
    }

    #[test]
    fn create_path_validation_rejects_a_symbolic_link_source() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let target = temporary.path().join("target");
        let link = temporary.path().join("linked-folder");
        std::fs::create_dir(&target).expect("source directory");
        match std::os::windows::fs::symlink_dir(&target, &link) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                return;
            }
            Err(error) => panic!("failed to create directory symlink: {error}"),
        }

        let error = validate_create_paths(vec![link])
            .expect_err("Explorer must disable creation for a linked source");
        assert_eq!(error.code(), E_FAIL);
    }

    #[test]
    fn create_command_state_matches_the_selection_limit() {
        assert!(!create_selection_enabled(0));
        assert!(create_selection_enabled(1));
        assert!(create_selection_enabled(MAX_SELECTED_ITEMS));
        assert!(!create_selection_enabled(MAX_SELECTED_ITEMS + 1));
    }

    #[test]
    fn dll_factory_creates_the_explorer_command() {
        let mut factory_raw = std::ptr::null_mut();
        let status =
            DllGetClassObject(&CREATE_COMMAND_CLSID, &IClassFactory::IID, &mut factory_raw);
        assert!(status.is_ok());
        assert!(!factory_raw.is_null());
        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let command: IExplorerCommand = unsafe {
            factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create IExplorerCommand")
        };
        assert_eq!(
            unsafe { command.GetCanonicalName() }.unwrap(),
            CREATE_COMMAND_CLSID
        );
        let title = unsafe { command.GetTitle(None::<&IShellItemArray>) }.unwrap();
        let title_text = unsafe { title.to_string() }.unwrap();
        unsafe { CoTaskMemFree(Some(title.0.cast())) };
        assert!(title_text.contains("ZiFile"));
    }

    #[test]
    fn create_command_exposes_a_site_interface_for_background_contexts() {
        let mut factory_raw = std::ptr::null_mut();
        let status =
            DllGetClassObject(&CREATE_COMMAND_CLSID, &IClassFactory::IID, &mut factory_raw);
        assert!(status.is_ok());
        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let command: IExplorerCommand = unsafe {
            factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create the create command")
        };
        let site: IObjectWithSite = command
            .cast()
            .expect("create command should expose IObjectWithSite");
        let error = unsafe { site.GetSite::<IUnknown>() }
            .expect_err("a command without an Explorer site should report no interface");
        assert_eq!(error.code(), E_NOINTERFACE);
    }

    #[test]
    fn extract_command_requires_a_valid_archive_file() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let directory = temporary.path().join("folder.zip");
        std::fs::create_dir(&directory).expect("archive-named directory");
        assert!(!extract_path_supported(&directory));

        let invalid = temporary.path().join("notes.zip");
        std::fs::write(&invalid, b"not a valid archive").expect("archive placeholder");
        assert!(!extract_path_supported(&invalid));

        let source = temporary.path().join("payload.txt");
        std::fs::write(&source, b"payload").expect("source payload");
        let renamed_archive = temporary.path().join("sample.bin");
        zifile_core::create_archive(
            &[source],
            &renamed_archive,
            zifile_core::ArchiveFormat::Zip,
            &zifile_core::CreateOptions::default(),
        )
        .expect("valid archive");
        assert!(extract_path_supported(&renamed_archive));
    }

    #[test]
    fn dll_factory_creates_the_extract_command() {
        let mut factory_raw = std::ptr::null_mut();
        let status = DllGetClassObject(
            &EXTRACT_COMMAND_CLSID,
            &IClassFactory::IID,
            &mut factory_raw,
        );
        assert!(status.is_ok());
        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let command: IExplorerCommand = unsafe {
            factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create extract command")
        };
        assert_eq!(
            unsafe { command.GetCanonicalName() }.unwrap(),
            EXTRACT_COMMAND_CLSID
        );
    }

    #[test]
    fn extract_state_defers_path_resolution_when_explorer_cannot_wait() {
        let mut factory_raw = std::ptr::null_mut();
        let status = DllGetClassObject(
            &EXTRACT_COMMAND_CLSID,
            &IClassFactory::IID,
            &mut factory_raw,
        );
        assert!(status.is_ok());
        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let command: IExplorerCommand = unsafe {
            factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create extract command")
        };
        let error = unsafe { command.GetState(None::<&IShellItemArray>, false) }
            .expect_err("Explorer should be told to retry state evaluation");
        assert_eq!(error.code(), E_PENDING);
    }

    #[test]
    fn create_state_defers_missing_selection_until_slow_state_pass() {
        let mut factory_raw = std::ptr::null_mut();
        let status =
            DllGetClassObject(&CREATE_COMMAND_CLSID, &IClassFactory::IID, &mut factory_raw);
        assert!(status.is_ok());
        let factory = unsafe { IClassFactory::from_raw(factory_raw) };
        let command: IExplorerCommand = unsafe {
            factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create the create command")
        };
        let error = unsafe { command.GetState(None::<&IShellItemArray>, false) }
            .expect_err("background state should be deferred when Explorer cannot wait");
        assert_eq!(error.code(), E_PENDING);
        let state = unsafe { command.GetState(None::<&IShellItemArray>, true) }
            .expect("missing site should produce a normal disabled state");
        assert_eq!(state, ECS_DISABLED.0 as u32);
    }

    #[test]
    fn command_icon_uses_the_sibling_desktop_resource() {
        assert_eq!(
            icon_resource_string(std::path::Path::new(
                r"C:\Program Files\ZiFile\zifile-desktop.exe"
            )),
            r"C:\Program Files\ZiFile\zifile-desktop.exe,0"
        );
    }
}
