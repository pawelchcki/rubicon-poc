FROM alpine as src

RUN apk add --no-cache curl 
RUN curl -L https://github.com/gothinkster/aspnetcore-realworld-example-app/archive/refs/heads/master.zip -o master.zip
RUN unzip master.zip -d /output_tmp
RUN rm master.zip
RUN mv /output_tmp/aspnetcore-realworld-example-app-master /src
WORKDIR /src


FROM mcr.microsoft.com/dotnet/runtime:8.0 AS base
WORKDIR /app

FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build
WORKDIR /src
COPY --from=src ["/src/build/build.csproj", "/src/build/"]

RUN dotnet restore "build/build.csproj"
RUN ls -lahr build/build.csproj; exit 1

COPY --from=src . .
WORKDIR "/src/build"
RUN dotnet build "build.csproj" -c Release -o /app/build

FROM build AS publish
RUN dotnet publish "build.csproj" -c Release -o /app/publish /p:UseAppHost=false

FROM publish AS final
WORKDIR /app
EXPOSE 5000

ENTRYPOINT ["dotnet", "Conduit.dll"]