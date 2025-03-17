FROM mcr.microsoft.com/dotnet/samples:aspnetapp

COPY --from=ghcr.io/pawelchcki/rebuildr/dotnet-poc:3.12.0 / /
ENTRYPOINT ["/bin/with_tracing.sh", "./aspnetapp"]