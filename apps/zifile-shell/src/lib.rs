#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_NOTIMPL, E_POINTER, HINSTANCE,
    HMODULE, S_FALSE,
};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::System::Com::{
    CoTaskMemAlloc, CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::UI::Shell::{
    ECS_DISABLED, ECS_ENABLED, IEnumExplorerCommand, IExplorerCommand, IExplorerCommand_Impl,
    IShellItemArray, SIGDN_FILESYSPATH,
};
use windows::core::{
    BOOL, GUID, HRESULT, IUnknown, Interface, PCWSTR, PWSTR, Ref, Result, implement,
};

pub const COMMAND_CLSID: GUID = GUID::from_u128(0x2f86f25d_3b76_4cd2_8fe8_9d7a2eefb531);
const MAX_SELECTED_ITEMS: u32 = 256;
const MAX_ARGUMENT_UTF16_UNITS: usize = 24_000;

#[implement(IExplorerCommand)]
struct ZiFileCommand;

impl IExplorerCommand_Impl for ZiFileCommand_Impl {
    fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        allocate_shell_string(if user_locale_is_chinese() {
            "使用 ZiFile 创建压缩文件"
        } else {
            "Create archive with ZiFile"
        })
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(COMMAND_CLSID)
    }

    fn GetState(&self, items: Ref<IShellItemArray>, _ok_to_be_slow: BOOL) -> Result<u32> {
        let enabled = items
            .as_ref()
            .and_then(|items| unsafe { items.GetCount().ok() })
            .is_some_and(|count| count > 0);
        Ok(if enabled {
            ECS_ENABLED.0
        } else {
            ECS_DISABLED.0
        } as u32)
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _bind_context: Ref<IBindCtx>) -> Result<()> {
        let items = items
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        let paths = collect_paths(items)?;
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

#[implement(IClassFactory)]
struct ZiFileClassFactory;

impl IClassFactory_Impl for ZiFileClassFactory_Impl {
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
        let unknown: IUnknown = ZiFileCommand.into();
        unsafe { unknown.query(iid, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(_instance: HINSTANCE, _reason: u32, _reserved: *mut c_void) -> BOOL {
    BOOL(1)
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if class_id.is_null() || interface_id.is_null() || object.is_null() {
        return E_POINTER;
    }
    if unsafe { *class_id } != COMMAND_CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = ZiFileClassFactory.into();
    unsafe { factory.query(interface_id, object) }
}

fn collect_paths(items: &IShellItemArray) -> Result<Vec<PathBuf>> {
    let count = unsafe { items.GetCount()? };
    if count == 0 || count > MAX_SELECTED_ITEMS {
        return Err(E_FAIL.into());
    }
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = unsafe { items.GetItemAt(index)? };
        let display_name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        let path_result = unsafe { display_name.to_string() }.map(PathBuf::from);
        unsafe { CoTaskMemFree(Some(display_name.0.cast())) };
        let path = path_result?;
        paths.push(path);
        if !command_line_within_limit(&paths) {
            return Err(E_FAIL.into());
        }
    }
    Ok(paths)
}

fn command_line_within_limit(paths: &[PathBuf]) -> bool {
    let units = paths
        .iter()
        .fold("--create".encode_utf16().count(), |total, path| {
            total
                .saturating_add(path.as_os_str().encode_wide().count())
                .saturating_add(3)
        });
    units <= MAX_ARGUMENT_UTF16_UNITS
}

fn sibling_desktop_path() -> Result<PathBuf> {
    let mut module = HMODULE::default();
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(DllGetClassObject as *const () as *const u16),
            &mut module,
        )?;
    }
    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetModuleFileNameW(Some(module), &mut buffer) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(E_FAIL.into());
    }
    let mut path = PathBuf::from(String::from_utf16_lossy(&buffer[..length]));
    path.set_file_name("zifile-desktop.exe");
    Ok(path)
}

fn user_locale_is_chinese() -> bool {
    let mut locale = [0_u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut locale) };
    length > 2 && String::from_utf16_lossy(&locale[..length as usize - 1]).starts_with("zh")
}

fn allocate_shell_string(value: &str) -> Result<PWSTR> {
    let mut wide = value.encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    let bytes = wide.len() * size_of::<u16>();
    let destination = unsafe { CoTaskMemAlloc(bytes) }.cast::<u16>();
    if destination.is_null() {
        return Err(E_FAIL.into());
    }
    unsafe { destination.copy_from_nonoverlapping(wide.as_ptr(), wide.len()) };
    Ok(PWSTR(destination))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_clsid_is_stable() {
        assert_eq!(
            COMMAND_CLSID,
            GUID::from_u128(0x2f86f25d_3b76_4cd2_8fe8_9d7a2eefb531)
        );
    }

    #[test]
    fn command_line_budget_accepts_unicode_and_rejects_oversized_selection() {
        assert!(command_line_within_limit(&[
            PathBuf::from(r"C:\资料\甲.txt"),
            PathBuf::from(r"C:\资料\乙 folder"),
        ]));
        assert!(!command_line_within_limit(&[PathBuf::from(
            "x".repeat(MAX_ARGUMENT_UTF16_UNITS)
        )]));
    }

    #[test]
    fn dll_factory_creates_the_explorer_command() {
        let mut factory_raw = std::ptr::null_mut();
        let status = DllGetClassObject(&COMMAND_CLSID, &IClassFactory::IID, &mut factory_raw);
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
            COMMAND_CLSID
        );
        let title = unsafe { command.GetTitle(None::<&IShellItemArray>) }.unwrap();
        let title_text = unsafe { title.to_string() }.unwrap();
        unsafe { CoTaskMemFree(Some(title.0.cast())) };
        assert!(title_text.contains("ZiFile"));
    }
}
