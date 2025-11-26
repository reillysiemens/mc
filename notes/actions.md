# Fetch
```bash
mc fetch
```
- FetchManifest
- GetLatestReleaseURL
- OpenTargetFile
- StreamDownload
- CloseTargetFile

```bash
mc fetch --snapshot
```
- FetchManifest
- GetLatestSnapshotURL
- OpenTargetFile
- StreamDownload
- CloseTargetFile

```bash
mc fetch 1.18.1
```
- FetchManifest
- GetVersionURL
- OpenTargetFile
- StreamDownload
- CloseTargetFile

```bash
mc fetch --list
```
- FetchManifest
- OutputVersions


# Others
```bash
mc run '/op heavymetalpanda'
mc launch /usr/bin/java -Xmx2048M -Xms2048M -jar ./server.jar --nogui
mc attach
```
