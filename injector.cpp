#include <Windows.h>
#include <TlHelp32.h>
#include <iostream>

struct NtCreateThreadExBuffer
{
    SIZE_T Size;
    SIZE_T Unknown1;
    SIZE_T Unknown2;
    PULONG Unknown3;
    SIZE_T Unknown4;
    SIZE_T Unknown5;
    SIZE_T Unknown6;
    PULONG Unknown7;
    SIZE_T Unknown8;
};

#pragma comment(lib, "ntdll.lib")
EXTERN_C NTSYSAPI NTSTATUS NTAPI NtCreateThreadEx(PHANDLE,
    ACCESS_MASK, LPVOID, HANDLE, LPTHREAD_START_ROUTINE, LPVOID,
    BOOL, SIZE_T, SIZE_T, SIZE_T, LPVOID);

DWORD GetPid(const wchar_t *targetProcess)
{
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    PROCESSENTRY32 procEntry;
    procEntry.dwSize = sizeof(procEntry);

    if (snap && snap != INVALID_HANDLE_VALUE && Process32First(snap, &procEntry))
    {
        do
        {
            if (!wcscmp(procEntry.szExeFile, targetProcess))
            {
                break;
            }
        } while (Process32Next(snap, &procEntry));
    }
    CloseHandle(snap);
    return procEntry.th32ProcessID;
}

int main()
{
    DWORD dwPid = GetPid(L"Among Us.exe");
    NtCreateThreadExBuffer ntbuffer;

    memset(&ntbuffer, 0, sizeof(NtCreateThreadExBuffer));
    DWORD temp1 = 0;
    DWORD temp2 = 0;

    ntbuffer.Size = sizeof(NtCreateThreadExBuffer);
    ntbuffer.Unknown1 = 0x10003;
    ntbuffer.Unknown2 = 0x8;
    ntbuffer.Unknown3 = (DWORD *)&temp2;
    ntbuffer.Unknown4 = 0;
    ntbuffer.Unknown5 = 0x10004;
    ntbuffer.Unknown6 = 4;
    ntbuffer.Unknown7 = &temp1;
    ntbuffer.Unknown8 = 0;

    HANDLE proc = OpenProcess(GENERIC_ALL, 0, dwPid);
    HANDLE hThread;
    wchar_t path[] = L"C:\\Users\\Development\\Documents\\cheats\\Test\\Debug\\IL2CppDLL.dll";
    LPVOID allocAddr = VirtualAllocEx(proc, 0, sizeof(path), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    WriteProcessMemory(proc, allocAddr, path, sizeof(path), nullptr);
    NTSTATUS status = NtCreateThreadEx(&hThread, GENERIC_ALL, NULL, proc,
        (LPTHREAD_START_ROUTINE)GetProcAddress(GetModuleHandle(L"kernel32.dll"), "LoadLibraryW"), allocAddr,
        FALSE, NULL, NULL, NULL, &ntbuffer);

    return 0;
}