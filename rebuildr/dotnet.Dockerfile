FROM ghcr.io/datadog/dd-trace-dotnet/dd-lib-dotnet-init:3.12.0 as src
USER root
RUN mkdir -p /output/var/log/datadog/dotnet/
RUN chmod 777 /output/var/log/datadog/dotnet/

FROM scratch as collect

WORKDIR /opt/_auto_dd
COPY --from=src /datadog-init /opt/_auto_dd/
COPY --from=src /output/var/log/datadog/dotnet/ /var/log/datadog/dotnet/

COPY with_tracing.sh /bin/with_tracing.sh
COPY with_profiler.sh /bin/with_profiler.sh

FROM scratch as final
COPY --from=collect . .
