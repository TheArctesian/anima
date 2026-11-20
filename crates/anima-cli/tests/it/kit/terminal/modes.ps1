param([string]$Program, [Parameter(ValueFromRemainingArguments=$true)][string[]]$Words)
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ConsoleMode {
    [DllImport("kernel32.dll")] public static extern IntPtr GetStdHandle(int id);
    [DllImport("kernel32.dll", SetLastError=true)] public static extern bool GetConsoleMode(IntPtr handle, out uint mode);
    public static uint Read() {
        uint mode;
        if (!GetConsoleMode(GetStdHandle(-10), out mode)) throw new System.ComponentModel.Win32Exception();
        return mode;
    }
}
'@
[Console]::WriteLine("modes-before " + [ConsoleMode]::Read())
$child = Start-Process -FilePath ([System.IO.Path]::GetFullPath($Program)) -ArgumentList $Words -NoNewWindow -Wait -PassThru
$code = $child.ExitCode
$child.Dispose()
[Console]::WriteLine("child-exit " + $code)
[Console]::WriteLine("modes-after " + [ConsoleMode]::Read())
exit $code
