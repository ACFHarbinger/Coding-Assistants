// infra/supabase/supabase_config.js
// Supabase configuration and client initialization example.
//
// This is deliberately a future-facing scaffold. Configure a project URL and
// publishable/anon key through the runtime environment; do not place service
// role keys or user secrets in this file or in a desktop bundle.

import { createClient } from "@supabase/supabase-js";

const supabaseUrl = process.env.SUPABASE_URL;
const supabaseAnonKey = process.env.SUPABASE_ANON_KEY;

export const supabase = createClient(supabaseUrl, supabaseAnonKey);

// Convenience exports mirroring the Firebase Auth/Firestore service split.
export const auth = supabase.auth;
export const db = supabase;
export const storage = supabase.storage;

export default supabase;
