using System;
using System.Collections.Generic;
using System.IO;

namespace Tests
{
    class Program
    {
        static void Main(string[] args)
        {
            File.Copy("../../../../toolset/lib/debug/kaiju_compiler_capi.dll", "kaiju_compiler_capi.dll", true);
            File.Copy("../../../../toolset/lib/debug/kaiju_vm_capi.dll", "kaiju_vm_capi.dll", true);
            var files = new Dictionary<string, byte[]>
            {
                ["descriptor.kjo"] = File.ReadAllBytes("../../res/descriptor.kjo"),
                ["program.kj"] = File.ReadAllBytes("../../res/program.kj")
            };
            var result = Kaiju.Compiler.API.CompileBin(
                "program.kj",
                "descriptor.kjo",
                files,
                error => Console.Error.WriteLine(error)
            );
            Kaiju.VM.API.Run(
                result,
                "main",
                1024,
                1024,
                (op, paramsPtrs, targetsPtrs) =>
                {
                    if (op == "add")
                    {
                        var a = Kaiju.VM.API.StateLoad<int>(paramsPtrs[0]).Value;
                        var b = Kaiju.VM.API.StateLoad<int>(paramsPtrs[1]).Value;
                        Kaiju.VM.API.StateStore(targetsPtrs[0], a + b);
                    }
                    else if (op == "out")
                    {
                        var v = Kaiju.VM.API.StateLoad<int>(paramsPtrs[0]).Value;
                        Console.WriteLine("OUT: {0}", v);
                    }
                },
                error => Console.Error.WriteLine(error)
            );
        }
    }
}
