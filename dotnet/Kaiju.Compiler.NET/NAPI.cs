using System;
using System.Runtime.InteropServices;

namespace Kaiju.Compiler
{
    public static class NAPI
    {
        private const string LibName = "kaiju_compiler_capi";
        private const CallingConvention LibCall = CallingConvention.Cdecl;

        [UnmanagedFunctionPointer(LibCall)]
        public delegate IntPtr OnServeFile(
            IntPtr context,
            [MarshalAs(UnmanagedType.LPStr)]
            string path,
            ref UIntPtr outSize
        );

        [UnmanagedFunctionPointer(LibCall)]
        public delegate void OnResultFile(
            IntPtr context,
            IntPtr bytes,
            UIntPtr size
        );

        [UnmanagedFunctionPointer(LibCall)]
        public delegate void OnError(
            IntPtr context,
            [MarshalAs(UnmanagedType.LPStr)]
            string error
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_compile_program_pst", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool CompilePst(
            [MarshalAs(UnmanagedType.LPStr)]
            string inputPath,
            [MarshalAs(UnmanagedType.LPStr)]
            string opsdescPath,
            [MarshalAs(UnmanagedType.I1)]
            bool pretty,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnServeFile onServeFile,
            IntPtr serveContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnResultFile onResultFile,
            IntPtr resultContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );

        [DllImport(LibName, CallingConvention = LibCall, EntryPoint = "kaiju_compile_program_bin", CharSet = CharSet.Ansi)]
        [return: MarshalAs(UnmanagedType.I1)]
        public extern static bool CompileBin(
            [MarshalAs(UnmanagedType.LPStr)]
            string inputPath,
            [MarshalAs(UnmanagedType.LPStr)]
            string opsdescPath,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnServeFile onServeFile,
            IntPtr serveContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnResultFile onResultFile,
            IntPtr resultContext,
            [MarshalAs(UnmanagedType.FunctionPtr)]
            OnError onError,
            IntPtr errorContext
        );
    }
}
