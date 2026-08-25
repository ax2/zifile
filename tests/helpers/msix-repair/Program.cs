using System.Text.Json;
using Microsoft.Windows.Management.Deployment;

const int schemaVersion = 1;

static void WriteResult(object value)
{
    Console.WriteLine(JsonSerializer.Serialize(value, new JsonSerializerOptions
    {
        WriteIndented = true,
    }));
}

if (args.Length == 1 && (args[0] == "--probe" || args[0] == "--probe-child"))
{
    var supported = PackageDeploymentManager.IsPackageDeploymentFeatureSupported(
        PackageDeploymentFeature.RepairPackage);
    WriteResult(new
    {
        schema_version = schemaVersion,
        operation = "probe",
        probe_completed = true,
        probe_timed_out = false,
        repair_supported = supported,
        repair_semantics = "preserve_application_data",
    });
    return 0;
}

if (args.Length == 1 && args[0] == "--probe-child-hang")
{
    await Task.Delay(Timeout.InfiniteTimeSpan);
    return 0;
}

if (args.Length != 2 || args[0] != "--package-full-name" || string.IsNullOrWhiteSpace(args[1]))
{
    Console.Error.WriteLine("Usage: MsixRepair --probe | --package-full-name <PackageFullName>");
    return 2;
}

if (!PackageDeploymentManager.IsPackageDeploymentFeatureSupported(
        PackageDeploymentFeature.RepairPackage))
{
    Console.Error.WriteLine("Windows App SDK reports that package Repair is unsupported.");
    return 3;
}

var packageFullName = args[1];
var manager = PackageDeploymentManager.GetDefault();
var result = await manager.RepairPackageAsync(packageFullName);
var succeeded = result.Status == PackageDeploymentStatus.CompletedSuccess;

WriteResult(new
{
    schema_version = schemaVersion,
    operation = "repair",
    package_full_name = packageFullName,
    status = result.Status.ToString(),
    succeeded,
    error = $"0x{result.Error.HResult:X8}",
    extended_error = $"0x{result.ExtendedError.HResult:X8}",
    result.ErrorText,
    repair_semantics = "preserve_application_data",
});

return succeeded ? 0 : 1;
