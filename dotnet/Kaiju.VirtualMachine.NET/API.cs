using System;
using System.Runtime.InteropServices;

namespace Kaiju.VM
{
    public static class API
    {
        public delegate void OnError(string error);
        public delegate void OnProcessOp(string op, UIntPtr[] paramsPtrs, UIntPtr[] targetsPtrs);

        public static bool Run(byte[] bytes, string entry, uint memSize, uint stackSize, OnProcessOp onProcessOp, OnError onError = null)
        {
            IntPtr buffer = Marshal.AllocHGlobal(bytes.Length);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            var result = false;
            try
            {
                result = NAPI.Run(
                    buffer,
                    (UIntPtr)bytes.Length,
                    entry,
                    (UIntPtr)memSize,
                    (UIntPtr)stackSize,
                    (context, op, paramsPtrs, paramsCount, targetsPtrs, targetsCount) =>
                    {
                        onProcessOp?.Invoke(op, MakePointers(paramsPtrs, paramsCount), MakePointers(targetsPtrs, targetsCount));
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
                Marshal.FreeHGlobal(buffer);
            }
            return result;
        }

        public static UIntPtr Start(byte[] bytes, string entry, uint memSize, uint stackSize, OnError onError = null)
        {
            IntPtr buffer = Marshal.AllocHGlobal(bytes.Length);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            var result = UIntPtr.Zero;
            try
            {
                result = NAPI.Start(
                    buffer,
                    (UIntPtr)bytes.Length,
                    entry,
                    (UIntPtr)memSize,
                    (UIntPtr)stackSize,
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
                Marshal.FreeHGlobal(buffer);
            }
            return result;
        }

        public static bool Resume(UIntPtr handle, OnProcessOp onProcessOp, OnError onError = null)
        {
            var result = false;
            try
            {
                result = NAPI.Resume(
                    handle,
                    (context, op, paramsPtrs, paramsCount, targetsPtrs, targetsCount) =>
                    {
                        onProcessOp?.Invoke(op, MakePointers(paramsPtrs, paramsCount), MakePointers(targetsPtrs, targetsCount));
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
            return result;
        }

        public static bool Consume(UIntPtr handle, OnProcessOp onProcessOp, OnError onError = null)
        {
            var result = false;
            try
            {
                result = NAPI.Consume(
                    handle,
                    (context, op, paramsPtrs, paramsCount, targetsPtrs, targetsCount) =>
                    {
                        onProcessOp?.Invoke(op, MakePointers(paramsPtrs, paramsCount), MakePointers(targetsPtrs, targetsCount));
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
            return result;
        }

        public static void Cancel(UIntPtr handle)
        {
            NAPI.Cancel(handle);
        }

        public static UIntPtr Fork(UIntPtr handle, string entry, uint memSize, uint stackSize, OnError onError = null)
        {
            var result = UIntPtr.Zero;
            try
            {
                result = NAPI.Fork(
                    handle,
                    entry,
                    (UIntPtr)memSize,
                    (UIntPtr)stackSize,
                    (context, error) => onError?.Invoke(error),
                    IntPtr.Zero
                );
            }
            catch (Exception error)
            {
                onError?.Invoke(error.Message);
            }
            return result;
        }

        public static uint StateSize()
        {
            return (uint)NAPI.StateSize();
        }

        public static T? StateLoad<T>(UIntPtr address) where T : struct
        {
            if ((ulong)address + (ulong)Marshal.SizeOf<T>() <= (ulong)NAPI.StateSize())
            {
                return Marshal.PtrToStructure<T>(NAPI.StatePtr(address));
            }
            return null;
        }

        public static byte[] StateLoadBytes(UIntPtr address, int size)
        {
            if ((ulong)address + (ulong)size <= (ulong)NAPI.StateSize())
            {
                var result = new byte[size];
                Marshal.Copy(NAPI.StatePtrMut(address), result, 0, size);
                return result;
            }
            return null;
        }

        public static void StateStore<T>(UIntPtr address, T value) where T : struct
        {
            if ((ulong)address + (ulong)Marshal.SizeOf<T>() <= (ulong)NAPI.StateSize())
            {
                Marshal.StructureToPtr(value, NAPI.StatePtr(address), false);
            }
        }

        public static void StateStoreBytes(UIntPtr address, byte[] bytes)
        {
            if ((ulong)address + (ulong)bytes.Length <= (ulong)NAPI.StateSize())
            {
                Marshal.Copy(bytes, 0, NAPI.StatePtrMut(address), bytes.Length);
            }
        }

        public static bool ContextGoTo(string label)
        {
            return NAPI.ContextGoTo(label);
        }

        public static void ContextReturn()
        {
            NAPI.ContextReturn();
        }

        private static UIntPtr[] MakePointers(IntPtr table, UIntPtr count)
        {
            var result = new UIntPtr[(uint)count];
            unsafe
            {
                var size = sizeof(UIntPtr);
                for (var i = 0; i < (uint)count; ++i)
                {
                    result[i] = *(UIntPtr*)table.ToPointer();
                    table += size;
                }
            }
            return result;
        }
    }
}
