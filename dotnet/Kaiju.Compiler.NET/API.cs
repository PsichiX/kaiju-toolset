using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Kaiju.Compiler
{
    public static class API
    {
        public delegate void OnError(string error);

        public static byte[] CompilePst(string inputPath, string opsdescPath, bool pretty, Dictionary<string, byte[]> files, OnError onError = null)
        {
            var ptrs = new Dictionary<string, IntPtr>();
            byte[] result = null;
            try
            {
                NAPI.CompilePst(
                    inputPath,
                    opsdescPath,
                    pretty,
                    (IntPtr context, string path, ref UIntPtr outSize) =>
                    {
                        IntPtr ptr = IntPtr.Zero;
                        if (!ptrs.TryGetValue(path, out ptr) && files.TryGetValue(path, out byte[] file))
                        {
                            ptr = Marshal.AllocHGlobal(file.Length);
                            Marshal.Copy(file, 0, ptr, file.Length);
                            ptrs.Add(path, ptr);
                            outSize = (UIntPtr)file.Length;
                        }
                        return ptr;
                    },
                    IntPtr.Zero,
                    (context, bytes, size) =>
                    {
                        result = new byte[(int)size];
                        Marshal.Copy(bytes, result, 0, (int)size);
                    },
                    IntPtr.Zero,
                    (context, error) => onError?.Invoke(error),
                    IntPtr.Zero
                );
            }
            catch (Exception error)
            {
                onError?.Invoke(error.Message);
            }
            finally
            {
                foreach (var kv in ptrs)
                {
                    Marshal.FreeHGlobal(kv.Value);
                }
            }
            return result;
        }

        public static byte[] CompileBin(string inputPath, string opsdescPath, Dictionary<string, byte[]> files, OnError onError = null)
        {
            var ptrs = new Dictionary<string, IntPtr>(files.Count);
            byte[] result = null;
            try
            {
                NAPI.CompileBin(
                    inputPath,
                    opsdescPath,
                    (IntPtr context, string path, ref UIntPtr outSize) =>
                    {
                        IntPtr ptr = IntPtr.Zero;
                        if (!ptrs.TryGetValue(path, out ptr) && files.TryGetValue(path, out byte[] file))
                        {
                            ptr = Marshal.AllocHGlobal(file.Length);
                            Marshal.Copy(file, 0, ptr, file.Length);
                            ptrs.Add(path, ptr);
                            outSize = (UIntPtr)file.Length;
                        }
                        return ptr;
                    },
                    IntPtr.Zero,
                    (context, bytes, size) =>
                    {
                        result = new byte[(int)size];
                        Marshal.Copy(bytes, result, 0, (int)size);
                    },
                    IntPtr.Zero,
                    (context, error) => onError?.Invoke(error),
                    IntPtr.Zero
                );
            }
            catch (Exception error)
            {
                onError?.Invoke(error.Message);
            }
            finally
            {
                foreach (var kv in ptrs)
                {
                    Marshal.FreeHGlobal(kv.Value);
                }
            }
            return result;
        }
    }
}
