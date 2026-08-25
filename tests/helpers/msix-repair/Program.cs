using System.Diagnostics;
using System.Text.Json;
using Microsoft.Windows.Management.Deployment;

const int schemaVersion = 1;
const int probeTimeoutSeconds = 15;

static void WriteResult(object value)
{
    Console.WriteLine(JsonSerializer.Serialize(value, new JsonSerializerOptions
    {
        WriteIndented = true,
    }));
}

if (args.Length == 1 && args[0] == "--probe-child")
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

if (args.Length == 1 && (args[0] == "--probe" || args[0] == "--probe-timeout-fixture"))
{
    var isTimeoutFixture = args[0] == "--probe-timeout-fixture";
    var childArgument = isTimeoutFixture ? "--probe-child-hang" : "--probe-child";
    var timeout = TimeSpan.FromSeconds(isTimeoutFixture ? 1 : probeTimeoutSeconds);
    var executable = Environment.ProcessPath
        ?? throw new InvalidOperationException("Cannot resolve the Repair helper executable path.");
    using var child = new Process
    {
        StartInfo = new ProcessStartInfo(executable, childArgument)
        {
            CreateNoWindow = true,
            RedirectStandardError = true,
            RedirectStandardOutput = true,
            UseShellExecute = false,
        },
    };
    if (!child.Start())
    {
        throw new InvalidOperationException("Cannot start the isolated Repair probe.");
    }
    var outputTask = child.StandardOutput.ReadToEndAsync();
    var errorTask = child.StandardError.ReadToEndAsync();
    try
    {
        await child.WaitForExitAsync().WaitAsync(timeout);
    }
    catch (TimeoutException)
    {
        try
        {
            child.Kill();
        }
        catch (InvalidOperationException)
        {
            // The process exited between the timeout and the termination request.
        }
        WriteResult(new
        {
            schema_version = schemaVersion,
            operation = "probe",
            probe_completed = false,
            probe_timed_out = true,
            repair_supported = false,
            repair_semantics = "preserve_application_data",
        });
        return 0;
    }

    var output = await outputTask;
    if (child.ExitCode != 0)
    {
        _ = await errorTask;
        throw new InvalidOperationException($"The isolated Repair probe exited with code {child.ExitCode}.");
    }
    Console.Write(output);
    return child.ExitCode;
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
