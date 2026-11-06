// Coding-Assistants Unity bridge.
//
// Copy the `plugins/unity/` folder into a project's `Assets/` (or add it as
// a local package). `[InitializeOnLoad]` starts a localhost line-JSON TCP
// server; `EditorApplication.update` pumps queued jobs on the editor main
// thread (Unity editor APIs are main-thread only). Protocol:
//
//   {"op": "<tool>", "args": {...}}\n  ->  {"ok": true, "result": ...}\n
//                                    or  {"ok": false, "error": "..."}\n
//
// Tested target: Unity 2021.3+ (uses SceneManager, EditorSceneManager,
// AssetDatabase, GameObject.CreatePrimitive). No Newtonsoft dependency —
// a compact MiniJSON is embedded below.

using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace CodingAssistants
{
    [InitializeOnLoad]
    public static class Bridge
    {
        private const int Port = 9769;
        private static TcpListener _listener;
        private static Thread _accept;
        private static readonly ConcurrentQueue<Job> Jobs = new ConcurrentQueue<Job>();
        private static volatile bool _running;

        private class Job
        {
            public string Op;
            public Dictionary<string, object> Args;
            public readonly ManualResetEventSlim Done = new ManualResetEventSlim(false);
            public object Result;
            public string Error;
        }

        static Bridge()
        {
            Start();
            EditorApplication.quitting += Stop;
            AssemblyReloadEvents.beforeAssemblyReload += Stop;
        }

        private static void Start()
        {
            if (_running) return;
            try
            {
                _listener = new TcpListener(IPAddress.Loopback, Port);
                _listener.Start();
                _running = true;
                _accept = new Thread(AcceptLoop) { IsBackground = true };
                _accept.Start();
                EditorApplication.update += Pump;
                Debug.Log($"[coding-assistants] Unity bridge listening on 127.0.0.1:{Port}");
            }
            catch (Exception e)
            {
                Debug.LogError($"[coding-assistants] Unity bridge failed to start: {e.Message}");
            }
        }

        private static void Stop()
        {
            _running = false;
            try { _listener?.Stop(); } catch { /* ignore */ }
            EditorApplication.update -= Pump;
        }

        private static void AcceptLoop()
        {
            while (_running)
            {
                try
                {
                    var client = _listener.AcceptTcpClient();
                    ThreadPool.QueueUserWorkItem(_ => Serve(client));
                }
                catch { if (_running) Thread.Sleep(100); }
            }
        }

        private static void Serve(TcpClient client)
        {
            using (client)
            using (var stream = client.GetStream())
            {
                client.ReceiveTimeout = 60000;
                var buf = new List<byte>();
                var tmp = new byte[4096];
                try
                {
                    while (!buf.Contains((byte)'\n'))
                    {
                        int n = stream.Read(tmp, 0, tmp.Length);
                        if (n <= 0) return;
                        for (int i = 0; i < n; i++) buf.Add(tmp[i]);
                    }
                }
                catch { return; }

                int nl = buf.IndexOf((byte)'\n');
                var line = Encoding.UTF8.GetString(buf.ToArray(), 0, nl);
                var reply = Handle(line);
                var outBytes = Encoding.UTF8.GetBytes(MiniJSON.Serialize(reply) + "\n");
                try { stream.Write(outBytes, 0, outBytes.Length); } catch { /* ignore */ }
            }
        }

        private static Dictionary<string, object> Handle(string line)
        {
            Dictionary<string, object> msg;
            try { msg = MiniJSON.Deserialize(line) as Dictionary<string, object>; }
            catch (Exception e) { return Err($"invalid request JSON: {e.Message}"); }
            if (msg == null) return Err("request was not a JSON object");

            var job = new Job
            {
                Op = msg.TryGetValue("op", out var o) ? o as string : null,
                Args = msg.TryGetValue("args", out var a) ? a as Dictionary<string, object> : new Dictionary<string, object>(),
            };
            Jobs.Enqueue(job);
            if (!job.Done.Wait(120000)) return Err("op timed out on the editor main thread");
            return job.Error != null ? Err(job.Error) : Ok(job.Result);
        }

        private static void Pump()
        {
            if (!Jobs.TryDequeue(out var job)) return;
            try { job.Result = Ops.Run(job.Op, job.Args ?? new Dictionary<string, object>()); }
            catch (Exception e) { job.Error = $"{e.GetType().Name}: {e.Message}"; }
            finally { job.Done.Set(); }
        }

        private static Dictionary<string, object> Ok(object result) =>
            new Dictionary<string, object> { { "ok", true }, { "result", result } };

        private static Dictionary<string, object> Err(string msg) =>
            new Dictionary<string, object> { { "ok", false }, { "error", msg } };
    }

    // ------------------------------------------------------------- ops

    internal static class Ops
    {
        public static object Run(string op, Dictionary<string, object> a)
        {
            switch (op)
            {
                case "get_editor_summary": return GetEditorSummary();
                case "list_gameobjects": return ListGameObjects(GetInt(a, "limit", 300));
                case "create_gameobject": return CreateGameObject(a);
                case "delete_gameobject": return DeleteGameObject(GetStr(a, "path"));
                case "set_transform": return SetTransform(a);
                case "add_component": return AddComponent(GetStr(a, "path"), GetStr(a, "component"));
                case "list_assets": return ListAssets(GetStr(a, "folder", "Assets"), GetStr(a, "filter", ""));
                case "save_scene": return SaveScene();
                case "execute_menu_item": return ExecuteMenuItem(GetStr(a, "menu_path"));
                default: throw new Exception($"unknown op {op}");
            }
        }

        private static object GetEditorSummary()
        {
            var scene = SceneManager.GetActiveScene();
            return new Dictionary<string, object>
            {
                { "unity_version", Application.unityVersion },
                { "scene_name", scene.name },
                { "scene_path", scene.path },
                { "root_count", scene.rootCount },
                { "is_playing", EditorApplication.isPlaying },
            };
        }

        private static object ListGameObjects(int limit)
        {
            var list = new List<object>();
            foreach (var root in SceneManager.GetActiveScene().GetRootGameObjects())
                Walk(root.transform, "", list, limit);
            return list;
        }

        private static void Walk(Transform t, string prefix, List<object> outList, int limit)
        {
            if (outList.Count >= limit) return;
            var path = prefix == "" ? t.name : prefix + "/" + t.name;
            var comps = new List<object>();
            foreach (var c in t.GetComponents<Component>())
                if (c != null) comps.Add(c.GetType().Name);
            outList.Add(new Dictionary<string, object>
            {
                { "name", t.name }, { "path", path },
                { "active", t.gameObject.activeSelf }, { "components", comps },
            });
            foreach (Transform child in t) Walk(child, path, outList, limit);
        }

        private static Transform Resolve(string path)
        {
            if (string.IsNullOrEmpty(path)) return null;
            var go = GameObject.Find(path);
            if (go == null) throw new Exception($"GameObject '{path}' not found");
            return go.transform;
        }

        private static object CreateGameObject(Dictionary<string, object> a)
        {
            var name = GetStr(a, "name", null);
            GameObject go;
            var prim = GetStr(a, "primitive", null);
            if (!string.IsNullOrEmpty(prim))
                go = GameObject.CreatePrimitive((PrimitiveType)Enum.Parse(typeof(PrimitiveType), prim));
            else
                go = new GameObject();
            if (!string.IsNullOrEmpty(name)) go.name = name;
            var parent = GetStr(a, "parent", null);
            if (!string.IsNullOrEmpty(parent)) go.transform.SetParent(Resolve(parent), false);
            if (a.TryGetValue("position", out var p)) go.transform.localPosition = Vec3(p);
            Undo.RegisterCreatedObjectUndo(go, "CA create_gameobject");
            EditorSceneManager.MarkSceneDirty(go.scene);
            return new Dictionary<string, object> { { "path", PathOf(go.transform) } };
        }

        private static object DeleteGameObject(string path)
        {
            var t = Resolve(path);
            UnityEngine.Object.DestroyImmediate(t.gameObject);
            return new Dictionary<string, object> { { "deleted", path } };
        }

        private static object SetTransform(Dictionary<string, object> a)
        {
            var t = Resolve(GetStr(a, "path"));
            if (a.TryGetValue("position", out var p)) t.localPosition = Vec3(p);
            if (a.TryGetValue("rotation", out var r)) t.localEulerAngles = Vec3(r);
            if (a.TryGetValue("scale", out var s)) t.localScale = Vec3(s);
            EditorSceneManager.MarkSceneDirty(t.gameObject.scene);
            return new Dictionary<string, object> { { "path", PathOf(t) } };
        }

        private static object AddComponent(string path, string component)
        {
            var t = Resolve(path);
            var type = FindType(component);
            if (type == null) throw new Exception($"component type '{component}' not found");
            t.gameObject.AddComponent(type);
            return new Dictionary<string, object> { { "added", component }, { "path", path } };
        }

        private static object ListAssets(string folder, string filter)
        {
            var search = string.IsNullOrEmpty(filter) ? "" : filter;
            var guids = AssetDatabase.FindAssets(search, new[] { folder });
            var paths = new List<object>();
            foreach (var g in guids) paths.Add(AssetDatabase.GUIDToAssetPath(g));
            return paths;
        }

        private static object SaveScene()
        {
            var scene = SceneManager.GetActiveScene();
            EditorSceneManager.SaveScene(scene);
            return new Dictionary<string, object> { { "saved", scene.path } };
        }

        private static object ExecuteMenuItem(string menuPath)
        {
            var found = EditorApplication.ExecuteMenuItem(menuPath);
            return new Dictionary<string, object> { { "menu_path", menuPath }, { "found", found } };
        }

        // ----- helpers

        private static string PathOf(Transform t)
        {
            var parts = new List<string>();
            while (t != null) { parts.Insert(0, t.name); t = t.parent; }
            return string.Join("/", parts);
        }

        private static Vector3 Vec3(object o)
        {
            var l = (List<object>)o;
            return new Vector3(ToF(l[0]), ToF(l[1]), ToF(l[2]));
        }

        private static float ToF(object o) => Convert.ToSingle(o, CultureInfo.InvariantCulture);

        private static string GetStr(Dictionary<string, object> a, string k, string def = null) =>
            a.TryGetValue(k, out var v) && v != null ? v.ToString() : def;

        private static int GetInt(Dictionary<string, object> a, string k, int def) =>
            a.TryGetValue(k, out var v) && v != null ? Convert.ToInt32(v, CultureInfo.InvariantCulture) : def;

        private static Type FindType(string name)
        {
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
            {
                var t = asm.GetType(name) ?? asm.GetType("UnityEngine." + name);
                if (t != null && typeof(Component).IsAssignableFrom(t)) return t;
            }
            return null;
        }
    }

    // ---------------------------------------------- MiniJSON (compact)
    // Public-domain-style minimal JSON, enough for {op, args} in and a
    // reply out. Handles object / array / string / number / bool / null.

    internal static class MiniJSON
    {
        public static object Deserialize(string json)
        {
            int idx = 0;
            return ParseValue(json, ref idx);
        }

        private static void SkipWs(string s, ref int i)
        {
            while (i < s.Length && char.IsWhiteSpace(s[i])) i++;
        }

        private static object ParseValue(string s, ref int i)
        {
            SkipWs(s, ref i);
            char c = s[i];
            if (c == '{') return ParseObject(s, ref i);
            if (c == '[') return ParseArray(s, ref i);
            if (c == '"') return ParseString(s, ref i);
            if (c == 't') { i += 4; return true; }
            if (c == 'f') { i += 5; return false; }
            if (c == 'n') { i += 4; return null; }
            return ParseNumber(s, ref i);
        }

        private static Dictionary<string, object> ParseObject(string s, ref int i)
        {
            var d = new Dictionary<string, object>();
            i++; // {
            SkipWs(s, ref i);
            if (s[i] == '}') { i++; return d; }
            while (true)
            {
                SkipWs(s, ref i);
                var key = ParseString(s, ref i);
                SkipWs(s, ref i);
                i++; // :
                d[key] = ParseValue(s, ref i);
                SkipWs(s, ref i);
                if (s[i] == ',') { i++; continue; }
                i++; // }
                return d;
            }
        }

        private static List<object> ParseArray(string s, ref int i)
        {
            var l = new List<object>();
            i++; // [
            SkipWs(s, ref i);
            if (s[i] == ']') { i++; return l; }
            while (true)
            {
                l.Add(ParseValue(s, ref i));
                SkipWs(s, ref i);
                if (s[i] == ',') { i++; continue; }
                i++; // ]
                return l;
            }
        }

        private static string ParseString(string s, ref int i)
        {
            var sb = new StringBuilder();
            i++; // opening quote
            while (s[i] != '"')
            {
                if (s[i] == '\\')
                {
                    i++;
                    switch (s[i])
                    {
                        case 'n': sb.Append('\n'); break;
                        case 't': sb.Append('\t'); break;
                        case 'r': sb.Append('\r'); break;
                        case 'u':
                            sb.Append((char)Convert.ToInt32(s.Substring(i + 1, 4), 16));
                            i += 4;
                            break;
                        default: sb.Append(s[i]); break;
                    }
                }
                else sb.Append(s[i]);
                i++;
            }
            i++; // closing quote
            return sb.ToString();
        }

        private static object ParseNumber(string s, ref int i)
        {
            int start = i;
            while (i < s.Length && (char.IsDigit(s[i]) || "-+.eE".IndexOf(s[i]) >= 0)) i++;
            return double.Parse(s.Substring(start, i - start), CultureInfo.InvariantCulture);
        }

        public static string Serialize(object o)
        {
            var sb = new StringBuilder();
            Write(o, sb);
            return sb.ToString();
        }

        private static void Write(object o, StringBuilder sb)
        {
            switch (o)
            {
                case null: sb.Append("null"); break;
                case bool b: sb.Append(b ? "true" : "false"); break;
                case string s: WriteString(s, sb); break;
                case IDictionary d:
                    sb.Append('{');
                    bool firstD = true;
                    foreach (DictionaryEntry e in d)
                    {
                        if (!firstD) sb.Append(',');
                        firstD = false;
                        WriteString(e.Key.ToString(), sb);
                        sb.Append(':');
                        Write(e.Value, sb);
                    }
                    sb.Append('}');
                    break;
                case IEnumerable en:
                    sb.Append('[');
                    bool firstA = true;
                    foreach (var item in en)
                    {
                        if (!firstA) sb.Append(',');
                        firstA = false;
                        Write(item, sb);
                    }
                    sb.Append(']');
                    break;
                default:
                    sb.Append(Convert.ToString(o, CultureInfo.InvariantCulture));
                    break;
            }
        }

        private static void WriteString(string s, StringBuilder sb)
        {
            sb.Append('"');
            foreach (var c in s)
            {
                switch (c)
                {
                    case '"': sb.Append("\\\""); break;
                    case '\\': sb.Append("\\\\"); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    default:
                        if (c < ' ') sb.Append("\\u").Append(((int)c).ToString("x4"));
                        else sb.Append(c);
                        break;
                }
            }
            sb.Append('"');
        }
    }
}
