# Common test setup and utilities for hk Windows tests

BeforeAll {
    # Setup test environment
    $script:TestRoot = Join-Path $env:TEMP ("hk-test-" + [System.Guid]::NewGuid().ToString())
    New-Item -Path $script:TestRoot -ItemType Directory -Force | Out-Null

    # Always find the local hk.exe binary (try debug first, then release)
    $script:HkPath = Resolve-Path "target\debug\hk.exe" -ErrorAction SilentlyContinue
    if (-not $script:HkPath) {
        $script:HkPath = Resolve-Path "target\release\hk.exe" -ErrorAction SilentlyContinue
    }
    if (-not $script:HkPath) {
        $script:HkPath = Resolve-Path "..\..\target\debug\hk.exe" -ErrorAction SilentlyContinue
    }
    if (-not $script:HkPath) {
        $script:HkPath = Resolve-Path "..\..\target\release\hk.exe" -ErrorAction SilentlyContinue
    }
    if (-not $script:HkPath) {
        throw "Could not find hk.exe. Please build the project first."
    }

    # Determine how to run hk commands
    if ($env:USE_MISE -eq "true") {
        # In CI, use mise to provide PKL but run local hk.exe
        $script:HkCommand = { param($args) & mise x -- $script:HkPath @args }
    } else {
        # Local development, use direct path to hk.exe
        $script:HkCommand = { param($args) & $script:HkPath @args }
    }

    # Export variables for use in test files
    $global:TestRoot = $script:TestRoot
    $global:HkPath = $script:HkPath
    $global:HkCommand = $script:HkCommand
}

BeforeEach {
    # Create a new test directory for each test
    $script:TestDir = New-Item -Path (Join-Path $global:TestRoot ([System.Guid]::NewGuid().ToString())) -ItemType Directory
    Push-Location $script:TestDir

    # Initialize git repository
    git init | Out-Null
    git config user.email "test@example.com" | Out-Null
    git config user.name "Test User" | Out-Null

    # Export for use in tests
    $global:TestDir = $script:TestDir
}

AfterEach {
    Pop-Location
    if (Test-Path $global:TestDir) {
        Remove-Item $global:TestDir -Recurse -Force
    }
}

AfterAll {
    if (Test-Path $global:TestRoot) {
        Remove-Item $global:TestRoot -Recurse -Force
    }
}