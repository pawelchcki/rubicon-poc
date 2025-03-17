#! /usr/bin/env nix
#! nix shell path:../../rebuildr --command rebuildr load-py

from rebuildr.descriptor import Descriptor, GlobInput, ImageTarget, Inputs, FileInput

image = Descriptor(
    targets=[
        ImageTarget(
            dockerfile="dotnet.Dockerfile",
            repository="ghcr.io/pawelchcki/rebuildr/dotnet-poc",
            tag="3.12.0",
        )
    ],
    inputs=Inputs(
        files=[
            GlobInput(
                pattern="librubicon_poc.so",
                root_dir = "../target/x86_64-unknown-linux-none/release/"
            ),
            GlobInput(
                pattern="with_*.sh",
            ),
            FileInput("dotnet.env"),
            FileInput("ld.so.preload"),
        ]
    ),
)