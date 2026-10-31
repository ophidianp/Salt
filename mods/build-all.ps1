# Builds every Salt mod for every supported Minecraft version folder in
# mods/ and copies the jars to the launcher's salt-mods folder, where
# Salt installs them into instances on launch.
#
#   powershell -ExecutionPolicy Bypass -File mods\build-all.ps1
#
# Optional: -Chlorine "C:\path\to\chlorine\1.0.8-26.3" also builds your
# Chlorine checkout and copies its jar next to the Salt mods (a local
# Chlorine jar wins over the GitHub release).

param(
    [string]$Chlorine = "",
    [string]$JavaHome = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$dest = Join-Path $env:APPDATA "SaltLauncher\salt-mods"

if ($JavaHome) { $env:JAVA_HOME = $JavaHome }
elseif (-not $env:JAVA_HOME) {
    $jdk = Get-ChildItem "$env:USERPROFILE\.jdks" -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
    if ($jdk) { $env:JAVA_HOME = $jdk.FullName }
}
Write-Host "Using JAVA_HOME=$env:JAVA_HOME"

# Each folder is one Minecraft version band (26.3, 1.21.11, 1.8.9 ...).
# A folder with a WIP file in it is still being written and is skipped.
Get-ChildItem $root -Directory | Where-Object { (Test-Path (Join-Path $_.FullName "settings.gradle")) -and -not (Test-Path (Join-Path $_.FullName "WIP")) } | ForEach-Object {
    $mc = $_.Name
    Write-Host "`n=== Building Salt mods for Minecraft $mc ===" -ForegroundColor Cyan
    Push-Location $_.FullName
    try {
        & .\gradlew.bat clean build --console=plain
        if ($LASTEXITCODE -ne 0) { throw "Build failed for $mc" }
    } finally { Pop-Location }

    # build.gradle sends output to %USERPROFILE%\SaltLauncherBuild\mods-build\<mc>\<mod>\libs
    $built = Join-Path $env:USERPROFILE "SaltLauncherBuild\mods-build\$mc"
    $jars = Get-ChildItem $built -Recurse -Filter "*.jar" |
        Where-Object { $_.DirectoryName -like "*\libs" -and $_.Name -notlike "*-sources.jar" }
    # targets.txt lists every Minecraft version a folder's jars support
    # (e.g. the 1.21.1 build also runs on 1.21); default is the folder name.
    $targetsFile = Join-Path $_.FullName "targets.txt"
    $targets = if (Test-Path $targetsFile) { Get-Content $targetsFile | Where-Object { $_ -and -not $_.StartsWith("#") } } else { @($mc) }
    foreach ($t in $targets) {
        $out = Join-Path $dest $t.Trim()
        New-Item -ItemType Directory -Force $out | Out-Null
        Get-ChildItem $out -Filter "salt*.jar" | Remove-Item
        $jars | ForEach-Object { Copy-Item $_.FullName $out }
        Write-Host "  -> $($jars.Count) jars for Minecraft $t"
    }
}

if ($Chlorine) {
    Write-Host "`n=== Building Chlorine ($Chlorine) ===" -ForegroundColor Cyan
    Push-Location $Chlorine
    try {
        & .\gradlew.bat clean build --console=plain
        if ($LASTEXITCODE -ne 0) { throw "Chlorine build failed" }
        $mc = (Select-String -Path gradle.properties -Pattern "^minecraft_version=(.+)$").Matches[0].Groups[1].Value.Trim()
        $out = Join-Path $dest $mc
        New-Item -ItemType Directory -Force $out | Out-Null
        Get-ChildItem $out -Filter "chlorine*.jar" | Remove-Item
        Get-ChildItem build\libs -Filter "chlorine*.jar" | Where-Object { $_.Name -notlike "*-sources.jar" } |
            ForEach-Object { Copy-Item $_.FullName $out; Write-Host "  -> $($_.Name) (Minecraft $mc)" }
    } finally { Pop-Location }
}

Write-Host "`nDone. Jars are in $dest" -ForegroundColor Green
