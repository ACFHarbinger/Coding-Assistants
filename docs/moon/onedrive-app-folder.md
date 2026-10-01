# S12 OneDrive App Folder

Use Microsoft Graph `GET/PUT/DELETE /me/drive/special/approot`. That is the
hidden application folder (privacy equivalent of Drive `drive.appdata`).

Object names are only:

- `replica/[0-9a-f]{64}`
- `manifests/[0-9a-f]{64}`
- `devices/[0-9a-f]{32}/[0-9a-f]{64}`

Do not create sharing links, public folders, or `root:` paths. Conditional
writes use `If-None-Match: *` on create and `If-Match` on update. The replica
key stays local.
