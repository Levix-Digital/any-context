using System;
using System.IO;
using System.Diagnostics;

namespace AnyContext.Launcher
{
    class Program
    {
        private const string FALLBACK_VERSION = "v0.30.33";

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
                        string content = File.ReadAllText(versionFile).Trim().TrimStart('\uFEFF').Trim();
                        if (!string.IsNullOrEmpty(content))
                        {
                            while (content.StartsWith("v", StringComparison.OrdinalIgnoreCase))
                            {
                                content = content.Substring(1);
                            }
                            version = "v" + content.Trim();
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

            // 2. Direct invocation to finalize a pending atomic update
            if (args.Length > 0 && args[0] == "--finalize-update")
            {
                string stagingDir = Path.Combine(baseDir, "actx_staging");
                string pendingFlag = Path.Combine(stagingDir, "pending_update.json");
                return FinalizePendingUpdate(baseDir, stagingDir, pendingFlag);
            }

            // 3. Pre-flight check: if a staged update is already pending from an earlier run, finalize it first
            string preStagingDir = Path.Combine(baseDir, "actx_staging");
            string prePendingFlag = Path.Combine(preStagingDir, "pending_update.json");
            if (File.Exists(prePendingFlag))
            {
                FinalizePendingUpdate(baseDir, preStagingDir, prePendingFlag);
            }

            // 4. Locate the full AnyContext core engine executable
            string coreExe = Path.Combine(baseDir, "actx-core.exe");
            if (!File.Exists(coreExe))
            {
                coreExe = Path.Combine(baseDir, "actx-core");
            }

            // In case a background updater or self-update swap is finalizing in another process,
            // retry checking for up to 1500ms before concluding the binary is missing.
            if (!File.Exists(coreExe))
            {
                for (int i = 0; i < 15; i++)
                {
                    System.Threading.Thread.Sleep(100);
                    coreExe = Path.Combine(baseDir, "actx-core.exe");
                    if (File.Exists(coreExe)) break;
                    coreExe = Path.Combine(baseDir, "actx-core");
                    if (File.Exists(coreExe)) break;
                }
            }

            // Fallback for development environments: try python with entrypoint
            if (!File.Exists(coreExe))
            {
                string devMain = Path.Combine(baseDir, "..", "main.py");
                if (File.Exists(devMain))
                {
                    return LaunchProcess(baseDir, "python", string.Format("\"{0}\" {1}", Path.GetFullPath(devMain), JoinArgs(args)));
                }

                Console.Error.WriteLine("❌ Error: AnyContext core engine ('actx-core.exe') not found in: " + baseDir);
                return 1;
            }

            return LaunchProcess(baseDir, coreExe, JoinArgs(args));
        }

        private static int LaunchProcess(string baseDir, string filename, string arguments)
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
                    int exitCode = proc.ExitCode;

                    // 5. Post-exit check: if an atomic update was prepared in staging, finalize it synchronously now
                    string stagingDir = Path.Combine(baseDir, "actx_staging");
                    string pendingFlag = Path.Combine(stagingDir, "pending_update.json");
                    if (File.Exists(pendingFlag) || exitCode == 42)
                    {
                        int res = FinalizePendingUpdate(baseDir, stagingDir, pendingFlag);
                        return res == 0 ? 0 : exitCode;
                    }

                    return exitCode;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine("❌ Error executing AnyContext: " + ex.Message);
                return 1;
            }
        }

        public static int FinalizePendingUpdate(string baseDir, string stagingDir, string pendingFlag)
        {
            try
            {
                Console.WriteLine("\n📦 Finalizing installation (atomic swap)...");

                // Wait briefly (up to 50ms) to ensure OS releases all file handles from dying core process
                System.Threading.Thread.Sleep(50);

                string targetExe = Path.Combine(baseDir, "actx-core.exe");
                string oldExe = Path.Combine(baseDir, "actx_old.exe");
                string internalDir = Path.Combine(baseDir, "_internal");
                string rnd = Guid.NewGuid().ToString("N").Substring(0, 8);
                string oldInternalDir = Path.Combine(baseDir, "_internal_old_" + rnd);
                string stagingInternal = Path.Combine(stagingDir, "_internal");
                string stagingCoreExe = Path.Combine(stagingDir, "actx-core.exe");

                // Parse target version from pending_update.json
                string newVersion = "";
                if (File.Exists(pendingFlag))
                {
                    try
                    {
                        string json = File.ReadAllText(pendingFlag);
                        int vIdx = json.IndexOf("\"version\":");
                        if (vIdx >= 0)
                        {
                            int q1 = json.IndexOf("\"", vIdx + 10);
                            int q2 = json.IndexOf("\"", q1 + 1);
                            if (q1 >= 0 && q2 > q1)
                            {
                                newVersion = json.Substring(q1 + 1, q2 - q1 - 1).Trim();
                            }
                        }
                    }
                    catch {}
                }

                // 1. Rename existing _internal directory to oldInternalDir (NTFS metadata move < 5ms)
                if (Directory.Exists(internalDir))
                {
                    for (int i = 0; i < 30; i++)
                    {
                        try
                        {
                            Directory.Move(internalDir, oldInternalDir);
                            break;
                        }
                        catch (IOException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                        catch (UnauthorizedAccessException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                    }
                }

                // 2. Move staged _internal to target_dir (NTFS metadata move < 5ms)
                if (Directory.Exists(stagingInternal))
                {
                    for (int i = 0; i < 15; i++)
                    {
                        try
                        {
                            Directory.Move(stagingInternal, internalDir);
                            break;
                        }
                        catch (IOException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                        catch (UnauthorizedAccessException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                    }
                }

                // 3. Rename existing actx-core.exe to oldExe
                if (File.Exists(targetExe))
                {
                    if (File.Exists(oldExe))
                    {
                        try { File.Delete(oldExe); } catch {}
                    }
                    try
                    {
                        File.Move(targetExe, oldExe);
                    }
                    catch {}
                }

                // 4. Move staged actx-core.exe to targetExe
                if (File.Exists(stagingCoreExe))
                {
                    for (int i = 0; i < 15; i++)
                    {
                        try
                        {
                            File.Move(stagingCoreExe, targetExe);
                            break;
                        }
                        catch (IOException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                        catch (UnauthorizedAccessException)
                        {
                            System.Threading.Thread.Sleep(100);
                        }
                    }
                }

                // 5. Move any other staged files (e.g. version.txt, README, etc.) into baseDir
                if (Directory.Exists(stagingDir))
                {
                    try
                    {
                        foreach (string f in Directory.GetFiles(stagingDir))
                        {
                            string name = Path.GetFileName(f);
                            if (name.Equals("pending_update.json", StringComparison.OrdinalIgnoreCase) ||
                                name.Equals("actx-core.exe", StringComparison.OrdinalIgnoreCase))
                            {
                                continue;
                            }
                            string dest = Path.Combine(baseDir, name);
                            try
                            {
                                if (File.Exists(dest)) File.Delete(dest);
                                File.Move(f, dest);
                            }
                            catch {}
                        }
                    }
                    catch {}
                }

                // 6. Update version.txt
                if (!string.IsNullOrEmpty(newVersion))
                {
                    string versionFile = Path.Combine(baseDir, "version.txt");
                    try
                    {
                        File.WriteAllText(versionFile, newVersion + "\n");
                    }
                    catch {}
                }

                // 7. Cleanup staging directory and old backups
                try { if (File.Exists(pendingFlag)) File.Delete(pendingFlag); } catch {}
                try { if (Directory.Exists(stagingDir)) Directory.Delete(stagingDir, true); } catch {}
                try { if (File.Exists(oldExe)) File.Delete(oldExe); } catch {}
                try { if (Directory.Exists(oldInternalDir)) Directory.Delete(oldInternalDir, true); } catch {}

                // Clean any other stale _internal_old* directories
                try
                {
                    foreach (string d in Directory.GetDirectories(baseDir, "_internal_old*"))
                    {
                        try { Directory.Delete(d, true); } catch {}
                    }
                }
                catch {}

                // 8. Verification: Ensure actx-core.exe exists and is accessible
                if (File.Exists(targetExe))
                {
                    Console.WriteLine("🎉 AnyContext successfully updated" + (string.IsNullOrEmpty(newVersion) ? "" : " to " + newVersion) + "!");
                    Console.WriteLine("👉 Run 'actx' or 'actx --tui' to start the updated version.\n");
                    return 0;
                }
                else
                {
                    Console.Error.WriteLine("❌ Error: Update finalized but actx-core.exe was not found in: " + baseDir);
                    return 1;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine("❌ Error finalizing update: " + ex.Message);
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
