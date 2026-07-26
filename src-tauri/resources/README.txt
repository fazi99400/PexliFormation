PexliFormation — bundled toolchain folder
=========================================

For a fully offline installer, place a copy of the Solana/Agave
platform-tools here so the app never needs the internet:

    resources/
      platform-tools/
        bin/
          cargo-build-sbf(.exe)
          rustc, cargo, llvm ...

At runtime the app looks for:

    <app resources>/resources/platform-tools/bin/cargo-build-sbf.exe

first, before falling back to a system-installed toolchain on PATH.

If you leave this folder empty, the app still works as long as the SBF
toolchain is installed on the machine (see scripts/setup-toolchain.ps1),
and the toolchain badge in the app will offer to finish the one-time setup.
