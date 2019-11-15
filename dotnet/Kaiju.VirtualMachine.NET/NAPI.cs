using System;
using System.Runtime.InteropServices;

namespace Kaiju.VM
{
    public static class NAPI
    {
        private const string LibName = "kaiju_vm_capi";
        private const CallingConvention LibCall = CallingConvention.Cdecl;

        [StructLayout(LayoutKind.Sequential)]
        public struct KaijuInfoState
        {
            UIntPtr stackSize;
            UIntPtr memorySize;
            UIntPtr allSize;
            UIntPtr stackFree;
            UIntPtr memoryFree;
            UIntPtr allFree;
        }

        [UnmanagedFunctionPointer(LibCall)]
        public delegate void OnProcessOp(
            IntPtr context,
            [MarshalAs(UnmanagedType.LPStr)]
            string op,
            IntPtr paramsPtr,
            UIntPtr paramsCount,
            IntPtr targetsPtr,
            UIntPtr targetsCount
        );

        [UnmanagedFunctionPointer(LibCall)]
        public delegate void OnError(
            IntPtr context,
            [MarshalAs(UnmanagedType.LPStr)]
            string error
        );

        [UnmanagedFunctionPointer(LibCall)]
        public delegate void OnPerform(IntPtr context);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_run_program", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool Run(
            IntPtr bytes,
            UIntPtr size,
            [MarshalAs(UnmanagedType.LPStr)]
            string entry,
            UIntPtr memSize,
            UIntPtr stackSize,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnProcessOp onProcessOp,
            IntPtr processorContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_start_program", CharSet = CharSet.Ansi)]
        public extern static UIntPtr Start(
            IntPtr bytes,
            UIntPtr size,
            [MarshalAs(UnmanagedType.LPStr)]
            string entry,
            UIntPtr memSize,
            UIntPtr stackSize,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_resume_program", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool Resume(
            UIntPtr handle,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnProcessOp onProcessOp,
            IntPtr processorContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_consume_program", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool Consume(
            UIntPtr handle,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnProcessOp onProcessOp,
            IntPtr processorContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_cancel_program", CharSet = CharSet.Ansi)]
        public extern static void Cancel(UIntPtr handle);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_fork_program", CharSet = CharSet.Ansi)]
        public extern static UIntPtr Fork(
            UIntPtr handle,
            [MarshalAs(UnmanagedType.LPStr)]
            string entry,
            UIntPtr memSize,
            UIntPtr stackSize,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_with_program", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool With(
            UIntPtr handle,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnPerform onPerform,
            IntPtr performContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_size", CharSet = CharSet.Ansi)]
        public extern static UIntPtr StateSize();

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_ptr", CharSet = CharSet.Ansi)]
        public extern static IntPtr StatePtr(UIntPtr address);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_ptr_mut", CharSet = CharSet.Ansi)]
        public extern static IntPtr StatePtrMut(UIntPtr address);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_info", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StateInfo(ref KaijuInfoState outState);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_alloc_stack", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StateAllocStack(
            UIntPtr size,
            ref UIntPtr outAddress
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_pop_stack", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StatePopStack(UIntPtr size);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_stack_address", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StateStackAddress(ref UIntPtr outAddress);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_alloc_memory", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StateAllocMemory(
            UIntPtr size,
            ref UIntPtr outAddress
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_state_dealloc_memory", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool StateDeallocMemory(UIntPtr address);

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_context_go_to", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool ContextGoTo(
            [MarshalAs(UnmanagedType.LPStr)]
            string label
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_context_return", CharSet = CharSet.Ansi)]
        public extern static void ContextReturn();
    }
}
