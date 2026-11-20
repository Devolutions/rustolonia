#include <cstdio>

extern "C" void* __stdcall EGL_GetProcAddress(const char* name);

int main()
{
    if (!EGL_GetProcAddress("eglGetDisplay"))
        return 1;
    std::puts("Static ANGLE entry point resolved.");
    return 0;
}
