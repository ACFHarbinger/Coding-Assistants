# S12 Dropbox app folder

Register the Dropbox app with **App Folder** access, not full Dropbox.
Paths are relative to that folder.

Object names are only:

- `/replica/[0-9a-f]{64}`
- `/manifests/[0-9a-f]{64}`
- `/devices/[0-9a-f]{32}/[0-9a-f]{64}`

Uploads use `mode=add` to create and `mode=update` with `rev` to replace.
Do not call sharing endpoints. The replica key stays local.
