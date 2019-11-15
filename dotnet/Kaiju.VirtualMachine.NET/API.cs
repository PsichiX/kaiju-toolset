using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Kaiju.VM
{
    public static class API
    {
        public delegate void OnError(string error);
        public delegate void OnProcessOp(string op, UIntPtr[] paramsPtrs, UIntPtr[] targetsPtrs);
        public delegate void OnPerform();

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

        public static bool With(UIntPtr handle, OnPerform onPerform, OnError onError = null)
        {
            var result = false;
            try
            {
                result = NAPI.With(
                    handle,
                    (context) => onPerform?.Invoke(),
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

        public static uint StateSize()
        {
            return (uint)NAPI.StateSize();
        }

        public static NAPI.KaijuInfoState StateInfo()
        {
            var info = new NAPI.KaijuInfoState();
            NAPI.StateInfo(ref info);
            return info;
        }

        public static UIntPtr? StateAllocStack(uint size)
        {
            UIntPtr address = UIntPtr.Zero;
            if (NAPI.StateAllocStack((UIntPtr)size, ref address))
            {
                return address;
            }
            return null;
        }

        public static bool StatePopStack(uint size)
        {
            return NAPI.StatePopStack((UIntPtr)size);
        }

        public static UIntPtr? StateStackAddress()
        {
            UIntPtr address = UIntPtr.Zero;
            if (NAPI.StateStackAddress(ref address))
            {
                return address;
            }
            return null;
        }

        public static UIntPtr? StateAllocMemory(uint size)
        {
            UIntPtr address = UIntPtr.Zero;
            if (NAPI.StateAllocMemory((UIntPtr)size, ref address))
            {
                return address;
            }
            return null;
        }

        public static bool StateDeallocMemory(UIntPtr address)
        {
            return NAPI.StateDeallocMemory(address);
        }

        public static T? StateLoad<T>(UIntPtr address) where T : struct
        {
            if ((ulong)address + (ulong)Marshal.SizeOf<T>() <= (ulong)NAPI.StateSize())
            {
                return Marshal.PtrToStructure<T>(NAPI.StatePtr(address));
            }
            return null;
        }

        public static string StateLoad(UIntPtr address)
        {
            var p = StateLoad<IntPtr>(address);
            if (p.HasValue)
            {
                return Marshal.PtrToStringAuto(p.Value);
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

        public static void StateStore(UIntPtr address, string value, bool wrap = true)
        {
            if (value == null)
            {
                return;
            }
            var c = Encoding.UTF8.GetByteCount(value);
            var bytes = new byte[c + 1];
            Encoding.UTF8.GetBytes(value, 0, value.Length, bytes, 0);
            bytes[bytes.Length - 1] = 0;
            if (wrap)
            {
                var addr = StateAllocStack((uint)bytes.Length);
                if (addr.HasValue)
                {
                    StateStoreBytes(addr.Value, bytes);
                    StateStore(address, addr.Value);
                }
            }
            else
            {
                StateStoreBytes(address, bytes);
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
