using System;
using System.IO;
using System.Diagnostics;

namespace AnyContext.Launcher
{
    class Program
    {
        private const string FALLBACK_VERSION = "v0.28.79";

        static int Main(string[] args)
        {
            string baseDir = AppDomain.CurrentDomain.BaseDirectory;

            // 1. Ultra-fast path for version check (< 3ms)
            if (args.Length > 0 && (args[0] == "-v" || args[0] == "--version"))
            {
                string versionFile = Path.Combine(baseDir, "version.txt");
                string version = FALLBACK_VERSION;

                if (File.Exists(versionFile))
                {
                    try
                    {
                        string content = File.ReadAllText(versionFile).Trim();
                        if (!string.IsNullOrEmpty(content))
                        {
                            version = content.StartsWith("v") ? content : "v" + content;
                        }
                    }
                    catch
                    {
                        // Fallback to embedded version
                    }
                }

                Console.WriteLine(version);
                return 0;
            }

            // 2. Locate the full AnyContext core engine executable
            string coreExe = Path.Combine(baseDir, "actx-core.exe");
            if (!File.Exists(coreExe))
            {
                coreExe = Path.Combine(baseDir, "actx-core");
            }

            // Fallback for development environments: try python with entrypoint
            if (!File.Exists(coreExe))
            {
                string devMain = Path.Combine(baseDir, "..", "main.py");
                if (File.Exists(devMain))
                {
                    return LaunchProcess("python", string.Format("\"{0}\" {1}", Path.GetFullPath(devMain), JoinArgs(args)));
                }

                Console.Error.WriteLine("❌ Error: AnyContext core engine ('actx-core.exe') not found in: " + baseDir);
                return 1;
            }

            return LaunchProcess(coreExe, JoinArgs(args));
        }

        private static int LaunchProcess(string filename, string arguments)
        {
            try
            {
                ProcessStartInfo psi = new ProcessStartInfo
                {
                    FileName = filename,
                    Arguments = arguments,
                    UseShellExecute = false
                };
                try
                {
                    psi.EnvironmentVariables["ACTX_LAUNCHER_PID"] = Process.GetCurrentProcess().Id.ToString();
                }
                catch {}

                using (Process proc = Process.Start(psi))
                {
                    if (proc == null)
                    {
                        Console.Error.WriteLine("❌ Error: Failed to start AnyContext process: " + filename);
                        return 1;
                    }

                    proc.WaitForExit();
                    return proc.ExitCode;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine("❌ Error executing AnyContext: " + ex.Message);
                return 1;
            }
        }

        private static string JoinArgs(string[] args)
        {
            if (args == null || args.Length == 0) return "";
            string[] escaped = new string[args.Length];
            for (int i = 0; i < args.Length; i++)
            {
                string a = args[i];
                if (a.Contains(" ") || a.Contains("\""))
                {
                    escaped[i] = "\"" + a.Replace("\"", "\\\"") + "\"";
                }
                else
                {
                    escaped[i] = a;
                }
            }
            return string.Join(" ", escaped);
        }
    }
}
