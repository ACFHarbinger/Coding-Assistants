-- S11: private Supabase Storage policy for Coding-Assistants encrypted blobs.
-- Replace `ca-sync` with the configured SUPABASE_STORAGE_BUCKET value before
-- applying. The bucket must be private; no anon or public policy is allowed.
-- The client writes only hashed BlobIds below approved prefixes.

insert into storage.buckets (id, name, public)
values ('ca-sync', 'ca-sync', false)
on conflict (id) do update set public = false;

create policy "ca sync authenticated owner reads hashed blobs"
on storage.objects for select to authenticated
using (
  bucket_id = 'ca-sync'
  and name ~ '^(replica|devices/[a-z0-9-]+|manifests|journals)/[0-9a-f]{64}$'
);

create policy "ca sync authenticated owner inserts hashed blobs"
on storage.objects for insert to authenticated
with check (
  bucket_id = 'ca-sync'
  and name ~ '^(replica|devices/[a-z0-9-]+|manifests|journals)/[0-9a-f]{64}$'
);

create policy "ca sync authenticated owner updates hashed blobs"
on storage.objects for update to authenticated
using (
  bucket_id = 'ca-sync'
  and name ~ '^(replica|devices/[a-z0-9-]+|manifests|journals)/[0-9a-f]{64}$'
)
with check (
  bucket_id = 'ca-sync'
  and name ~ '^(replica|devices/[a-z0-9-]+|manifests|journals)/[0-9a-f]{64}$'
);

create policy "ca sync authenticated owner deletes hashed blobs"
on storage.objects for delete to authenticated
using (
  bucket_id = 'ca-sync'
  and name ~ '^(replica|devices/[a-z0-9-]+|manifests|journals)/[0-9a-f]{64}$'
);
