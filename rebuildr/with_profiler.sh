#!/bin/sh
export CORECLR_ENABLE_PROFILING=1
export CORECLR_PROFILER={846F5F1C-F9AE-4B07-969E-05C26BC060D8}
export CORECLR_PROFILER_PATH=/opt/_auto_dd/package/Datadog.Trace.ClrProfiler.Native.so
export DD_DOTNET_TRACER_HOME=/opt/_auto_dd/package
export LD_PRELOAD=/opt/_auto_dd/package/continuousprofiler/Datadog.Linux.ApiWrapper.x64.so
export DD_PROFILING_ENABLED=1

exec "$@"
