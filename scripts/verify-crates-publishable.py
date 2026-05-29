#!/usr/bin/env python3
"""Fail if the workspace has no package publishable to crates.io."""

import json
import subprocess
import sys


def main() -> int:
    try:
        metadata_process = subprocess.run(
            [
                "cargo",
                "metadata",
                "--locked",
                "--format-version",
                "1",
                "--no-deps",
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr)
        return error.returncode

    metadata = json.loads(metadata_process.stdout)
    workspace_members = set(metadata.get("workspace_members", []))
    workspace_packages = [
        package
        for package in metadata.get("packages", [])
        if package.get("id") in workspace_members
    ]

    publishable = []
    blocked = []

    for package in workspace_packages:
        publish_config = package.get("publish")
        package_ref = f"{package.get('name')} {package.get('version')}"

        if publish_config is None:
            publishable.append(package_ref)
        elif publish_config == []:
            blocked.append(f"{package_ref}: publish = false")
        else:
            blocked.append(
                f"{package_ref}: publish is restricted to alternate registries: "
                f"{publish_config}"
            )

    if publishable:
        print("Publishable crates.io package(s):")
        for package_ref in publishable:
            print(f"- {package_ref}")
        return 0

    print(
        "No workspace package is publishable to crates.io. "
        "Remove `publish = false` or registry restrictions before running the "
        "publish action.",
        file=sys.stderr,
    )
    for blocked_package in blocked:
        print(f"- {blocked_package}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
