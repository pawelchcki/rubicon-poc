#! /usr/bin/env nix
#! nix shell github:pawelchcki/rebuildr/v0.1 --command rebuildr load-py

from rebuildr.descriptor import Descriptor, GlobInput, ImageTarget, Inputs

image = Descriptor(
    targets=[
        ImageTarget(
            dockerfile="example-dotnet-app.Dockerfile",
            repository="ghcr.io/pawelchcki/rebuildr/dotnet-app-poc",
            tag="latest",
        )
    ],
    inputs=Inputs(),
)