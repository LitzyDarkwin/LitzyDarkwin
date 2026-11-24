#if UNITY_EDITOR
using System;
using System.IO;
using Roguelite.CameraSystem;
using Roguelite.Controls;
using Roguelite.Juice;
using Roguelite.Player;
using Roguelite.Pooling;
using Roguelite.Stats;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.SceneManagement;
using UnityEngine.UI;
using Object = UnityEngine.Object;

namespace Roguelite.EditorTools
{
    /// <summary>
    /// Menü: Roguelite > Demo Sahnesini Kur
    ///
    /// ADIM 2'nin tüm kurulumunu otomatik yapar:
    ///  - Player / Enemy katmanları
    ///  - InputChannel, StatSheet asset'leri
    ///  - Placeholder sprite'lar (karakter, gölge, daire, halka, zemin, dash ikonu) -> kod ile çizilir
    ///  - GhostSprite prefab'ı
    ///  - Demo sahnesi: Player (tüm bileşenler bağlı), kamera takibi, zemin, içinden dash atılabilecek "düşman" hedefleri,
    ///    Canvas + Floating Joystick + Dash butonu, EventSystem
    /// Tekrar çalıştırılabilir: var olan asset'leri yeniden kullanır, sahneyi baştan oluşturur.
    /// </summary>
    public static class DemoSceneBuilder
    {
        private const string Root = "Assets/_Project";
        private const string ArtDir = Root + "/Art/Generated";
        private const string SoDir = Root + "/ScriptableObjects";
        private const string PrefabDir = Root + "/Prefabs";
        private const string SceneDir = Root + "/Scenes";
        private const string ScenePath = SceneDir + "/Demo.unity";

        [MenuItem("Roguelite/Demo Sahnesini Kur", priority = 0)]
        public static void Build()
        {
            if (!EditorSceneManager.SaveCurrentModifiedScenesIfUserWantsTo()) return;

            try
            {
                // ÖNCE sahneyi aç: NewScene (Single) kullanılmayan asset'leri bellekten atar; asset'leri bundan SONRA yükle.
                Scene scene = EditorSceneManager.NewScene(NewSceneSetup.DefaultGameObjects, NewSceneMode.Single);

                EditorUtility.DisplayProgressBar("Roguelite", "Klasörler ve katmanlar...", 0.1f);
                EnsureFolder(ArtDir);
                EnsureFolder(SoDir);
                EnsureFolder(PrefabDir);
                EnsureFolder(SceneDir);

                int playerLayer = EnsureLayer("Player");
                int enemyLayer = EnsureLayer("Enemy");

                EditorUtility.DisplayProgressBar("Roguelite", "Sprite'lar çiziliyor...", 0.3f);
                Sprite characterSprite = CreateSprite("Character", 128, 128, 128, new Vector2(0.5f, 0.02f), PaintCharacter);
                Sprite shadowSprite = CreateSprite("Shadow", 96, 40, 128, new Vector2(0.5f, 0.5f), PaintShadow);
                Sprite circleSprite = CreateSprite("Circle", 128, 128, 128, new Vector2(0.5f, 0.5f), PaintCircle);
                Sprite ringSprite = CreateSprite("Ring", 256, 256, 100, new Vector2(0.5f, 0.5f), PaintRing);
                Sprite groundSprite = CreateSprite("GroundTile", 64, 64, 32, new Vector2(0.5f, 0.5f), PaintGround, true);
                Sprite dashIconSprite = CreateSprite("DashIcon", 128, 128, 100, new Vector2(0.5f, 0.5f), PaintDashIcon);
                Sprite enemySprite = CreateSprite("EnemyBlob", 128, 128, 128, new Vector2(0.5f, 0.5f), PaintEnemy);

                Material spriteMat = FindUnlitSpriteMaterial();

                EditorUtility.DisplayProgressBar("Roguelite", "ScriptableObject'ler...", 0.5f);
                InputChannelSO inputChannel = LoadOrCreateAsset<InputChannelSO>(SoDir + "/InputChannel.asset");
                StatSheetSO statSheet = LoadOrCreateAsset<StatSheetSO>(SoDir + "/StatSheet_Player.asset");

                EditorUtility.DisplayProgressBar("Roguelite", "Ghost prefab...", 0.6f);
                GhostSprite ghostPrefab = CreateGhostPrefab(spriteMat);

                EditorUtility.DisplayProgressBar("Roguelite", "Sahne kuruluyor...", 0.75f);
                AssetDatabase.SaveAssets();
                // Diskten taze referans al (bellekten atılmış olma ihtimaline karşı)
                inputChannel = AssetDatabase.LoadAssetAtPath<InputChannelSO>(SoDir + "/InputChannel.asset");
                statSheet = AssetDatabase.LoadAssetAtPath<StatSheetSO>(SoDir + "/StatSheet_Player.asset");
                ghostPrefab = AssetDatabase.LoadAssetAtPath<GhostSprite>(PrefabDir + "/GhostSprite.prefab");
                if (inputChannel == null || statSheet == null || ghostPrefab == null)
                    throw new InvalidOperationException("InputChannel / StatSheet / GhostSprite asset'leri oluşturulamadı.");

                // ---------- Sistemler
                var systems = new GameObject("[Systems]");
                systems.AddComponent<PoolManager>();
                var keyboard = systems.AddComponent<KeyboardInputBridge>();
                SetRef(keyboard, "inputChannel", inputChannel);

                // ---------- Zemin
                var ground = new GameObject("Ground");
                var groundSr = ground.AddComponent<SpriteRenderer>();
                groundSr.sprite = groundSprite;
                groundSr.drawMode = SpriteDrawMode.Tiled;
                groundSr.size = new Vector2(80f, 80f);
                groundSr.sortingOrder = -100;
                if (spriteMat != null) groundSr.sharedMaterial = spriteMat;

                // ---------- Player
                GameObject player = BuildPlayer(playerLayer, enemyLayer, characterSprite, shadowSprite, spriteMat,
                                                inputChannel, statSheet, ghostPrefab);

                // ---------- Dash ile içinden geçilecek hedefler (henüz yapay zekâ yok, ADIM 3)
                BuildDummyEnemies(enemyLayer, enemySprite, spriteMat);

                // ---------- Kamera
                Camera cam = Camera.main;
                if (cam == null)
                {
                    cam = new GameObject("Main Camera").AddComponent<Camera>();
                    cam.tag = "MainCamera";
                }
                cam.orthographic = true;
                cam.orthographicSize = 7f;
                cam.clearFlags = CameraClearFlags.SolidColor;
                cam.backgroundColor = new Color(0.07f, 0.07f, 0.1f);
                cam.transform.position = new Vector3(0f, 0f, -10f);
                if (!cam.TryGetComponent(out SmoothCameraFollow follow))
                    follow = cam.gameObject.AddComponent<SmoothCameraFollow>();
                SetRef(follow, "target", player.transform);

                // ---------- UI
                BuildUI(inputChannel, circleSprite, ringSprite, dashIconSprite);

                EditorUtility.DisplayProgressBar("Roguelite", "Kaydediliyor...", 0.95f);
                EditorSceneManager.SaveScene(scene, ScenePath);
                AddSceneToBuild(ScenePath);
                AssetDatabase.SaveAssets();

                Selection.activeGameObject = player;
                Debug.Log("<color=#7CFC00><b>[Roguelite]</b></color> Demo sahnesi hazır: " + ScenePath +
                          "  ▶ Play'e bas. WASD/Oklar = hareket, Space/Shift = dash. (Device Simulator'da joystick'i fareyle dene)");
            }
            catch (Exception e)
            {
                Debug.LogException(e);
                EditorUtility.DisplayDialog("Roguelite", "Kurulum sırasında hata oluştu. Console'a bak:\n" + e.Message, "Tamam");
            }
            finally
            {
                EditorUtility.ClearProgressBar();
            }
        }

        // =====================================================================================  PLAYER

        private static GameObject BuildPlayer(int playerLayer, int enemyLayer, Sprite character, Sprite shadow,
                                              Material mat, InputChannelSO input, StatSheetSO sheet, GhostSprite ghost)
        {
            var player = new GameObject("Player") { layer = playerLayer };

            var col = player.AddComponent<CircleCollider2D>();
            col.radius = 0.35f;
            col.offset = new Vector2(0f, 0.3f);

            var stats = player.AddComponent<CharacterStats>();
            SetRef(stats, "baseSheet", sheet);
            SetBool(stats, "broadcastChanges", true);

            var controller = player.AddComponent<PlayerController>(); // Rigidbody2D'yi RequireComponent ekler
            SetRef(controller, "inputChannel", input);
            SetInt(controller, "passThroughLayers", 1 << enemyLayer);

            var rb = player.GetComponent<Rigidbody2D>();
            rb.gravityScale = 0f;
            rb.freezeRotation = true;
            rb.interpolation = RigidbodyInterpolation2D.Interpolate;

            // Shadow (zıplamaz)
            var shadowGo = new GameObject("Shadow") { layer = playerLayer };
            shadowGo.transform.SetParent(player.transform, false);
            shadowGo.transform.localPosition = new Vector3(0f, 0.02f, 0f);
            var shadowSr = shadowGo.AddComponent<SpriteRenderer>();
            shadowSr.sprite = shadow;
            shadowSr.sortingOrder = 0;
            if (mat != null) shadowSr.sharedMaterial = mat;

            // Visual (juice)
            var visual = new GameObject("Visual") { layer = playerLayer };
            visual.transform.SetParent(player.transform, false);
            visual.AddComponent<ProceduralJuiceAnimator>();

            var spriteGo = new GameObject("Sprite") { layer = playerLayer };
            spriteGo.transform.SetParent(visual.transform, false);
            var sr = spriteGo.AddComponent<SpriteRenderer>();
            sr.sprite = character; // pivot ayakta -> ekstra offset gerekmez
            sr.sortingOrder = 10;
            if (mat != null) sr.sharedMaterial = mat;

            // Trail (opsiyonel parlak iz)
            var trailGo = new GameObject("Trail") { layer = playerLayer };
            trailGo.transform.SetParent(visual.transform, false);
            trailGo.transform.localPosition = new Vector3(0f, 0.45f, 0f);
            var trail = trailGo.AddComponent<TrailRenderer>();
            trail.time = 0.18f;
            trail.minVertexDistance = 0.05f;
            trail.widthCurve = new AnimationCurve(new Keyframe(0f, 0.55f), new Keyframe(1f, 0f));
            trail.colorGradient = DashGhostTrail.DefaultGradient();
            trail.emitting = false;
            trail.sortingOrder = 5;
            trail.sharedMaterial = mat != null ? mat : AssetDatabase.GetBuiltinExtraResource<Material>("Sprites-Default.mat");

            var ghostTrail = player.AddComponent<DashGhostTrail>();
            SetRef(ghostTrail, "sourceRenderer", sr);
            SetRef(ghostTrail, "ghostPrefab", ghost);
            SetRef(ghostTrail, "optionalTrail", trail);
            var so = new SerializedObject(ghostTrail);
            so.FindProperty("colorOverLifetime").gradientValue = DashGhostTrail.DefaultGradient();
            so.ApplyModifiedPropertiesWithoutUndo();

            return player;
        }

        private static void BuildDummyEnemies(int enemyLayer, Sprite sprite, Material mat)
        {
            var parent = new GameObject("DummyEnemies");
            var rnd = new System.Random(1234);
            for (int i = 0; i < 18; i++)
            {
                float angle = (float)(rnd.NextDouble() * Math.PI * 2.0);
                float dist = 3.5f + (float)rnd.NextDouble() * 10f;
                var go = new GameObject($"Dummy_{i:00}") { layer = enemyLayer };
                go.transform.SetParent(parent.transform, false);
                go.transform.position = new Vector3(Mathf.Cos(angle) * dist, Mathf.Sin(angle) * dist, 0f);
                float s = 0.8f + (float)rnd.NextDouble() * 0.5f;
                go.transform.localScale = new Vector3(s, s, 1f);

                var sr = go.AddComponent<SpriteRenderer>();
                sr.sprite = sprite;
                sr.sortingOrder = 5;
                if (mat != null) sr.sharedMaterial = mat;

                go.AddComponent<CircleCollider2D>().radius = 0.42f;
            }
        }

        // =====================================================================================  GHOST PREFAB

        private static GhostSprite CreateGhostPrefab(Material mat)
        {
            string path = PrefabDir + "/GhostSprite.prefab";
            var existing = AssetDatabase.LoadAssetAtPath<GhostSprite>(path);
            if (existing != null) return existing;

            var go = new GameObject("GhostSprite");
            var sr = go.AddComponent<SpriteRenderer>();
            if (mat != null) sr.sharedMaterial = mat;
            go.AddComponent<GhostSprite>();
            go.AddComponent<PooledObject>();
            PrefabUtility.SaveAsPrefabAsset(go, path);
            Object.DestroyImmediate(go);
            return AssetDatabase.LoadAssetAtPath<GhostSprite>(path);
        }

        // =====================================================================================  UI

        private static void BuildUI(InputChannelSO input, Sprite circle, Sprite ring, Sprite dashIcon)
        {
            var canvasGo = new GameObject("Canvas", typeof(RectTransform), typeof(Canvas), typeof(CanvasScaler), typeof(GraphicRaycaster));
            canvasGo.layer = LayerMask.NameToLayer("UI");
            var canvas = canvasGo.GetComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceOverlay;
            canvas.sortingOrder = 10;
            var scaler = canvasGo.GetComponent<CanvasScaler>();
            scaler.uiScaleMode = CanvasScaler.ScaleMode.ScaleWithScreenSize;
            scaler.referenceResolution = new Vector2(1080f, 1920f);
            scaler.matchWidthOrHeight = 0.5f;

            // ---- Joystick alanı (tam ekran, görünmez)
            RectTransform area = CreateUI("JoystickArea", canvasGo.transform);
            Stretch(area);
            var areaImg = area.gameObject.AddComponent<Image>();
            areaImg.color = new Color(1f, 1f, 1f, 0f);
            areaImg.raycastTarget = true;

            RectTransform bg = CreateUI("Background", area);
            bg.anchorMin = bg.anchorMax = new Vector2(0.5f, 0.5f);
            bg.sizeDelta = new Vector2(240f, 240f);
            bg.anchoredPosition = new Vector2(-280f, -620f); // idle konumu: sol alt
            var bgImg = bg.gameObject.AddComponent<Image>();
            bgImg.sprite = ring;
            bgImg.color = new Color(1f, 1f, 1f, 0.8f);
            bgImg.raycastTarget = false;

            RectTransform handle = CreateUI("Handle", bg);
            handle.anchorMin = handle.anchorMax = new Vector2(0.5f, 0.5f);
            handle.sizeDelta = new Vector2(110f, 110f);
            var hImg = handle.gameObject.AddComponent<Image>();
            hImg.sprite = circle;
            hImg.color = new Color(1f, 1f, 1f, 0.9f);
            hImg.raycastTarget = false;

            var joystick = area.gameObject.AddComponent<FloatingJoystick>();
            SetRef(joystick, "inputChannel", input);
            SetRef(joystick, "background", bg);
            SetRef(joystick, "handle", handle);

            // ---- Dash butonu (sağ alt)
            RectTransform dash = CreateUI("DashButton", canvasGo.transform);
            dash.anchorMin = dash.anchorMax = new Vector2(1f, 0f);
            dash.pivot = new Vector2(0.5f, 0.5f);
            dash.sizeDelta = new Vector2(220f, 220f);
            dash.anchoredPosition = new Vector2(-200f, 260f);
            var dashImg = dash.gameObject.AddComponent<Image>();
            dashImg.sprite = circle;
            dashImg.color = new Color(0.45f, 0.85f, 1f, 0.85f);
            dashImg.raycastTarget = true;

            RectTransform icon = CreateUI("Icon", dash);
            Stretch(icon, 45f);
            var iconImg = icon.gameObject.AddComponent<Image>();
            iconImg.sprite = dashIcon;
            iconImg.raycastTarget = false;

            RectTransform fill = CreateUI("CooldownFill", dash);
            Stretch(fill);
            var fillImg = fill.gameObject.AddComponent<Image>();
            fillImg.sprite = circle;
            fillImg.color = new Color(0f, 0f, 0f, 0.6f);
            fillImg.type = Image.Type.Filled;
            fillImg.fillMethod = Image.FillMethod.Radial360;
            fillImg.fillOrigin = (int)Image.Origin360.Top;
            fillImg.fillClockwise = false;
            fillImg.fillAmount = 0f;
            fillImg.raycastTarget = false;
            fillImg.enabled = false;

            RectTransform txt = CreateUI("ChargesText", dash);
            txt.anchorMin = txt.anchorMax = new Vector2(1f, 1f);
            txt.sizeDelta = new Vector2(80f, 80f);
            txt.anchoredPosition = new Vector2(-15f, -15f);
            var text = txt.gameObject.AddComponent<Text>();
            text.font = LoadBuiltinFont();
            text.fontSize = 48;
            text.fontStyle = FontStyle.Bold;
            text.alignment = TextAnchor.MiddleCenter;
            text.color = Color.white;
            text.text = "1";
            text.raycastTarget = false;

            var dashButton = dash.gameObject.AddComponent<DashButton>();
            SetRef(dashButton, "inputChannel", input);
            SetRef(dashButton, "iconImage", iconImg);
            SetRef(dashButton, "cooldownFill", fillImg);
            SetRef(dashButton, "chargesText", text);

            // ---- EventSystem (Input System varsa onun modülü)
            if (Object.FindAnyObjectByType<EventSystem>() == null)
            {
                var es = new GameObject("EventSystem", typeof(EventSystem));
                Type module = Type.GetType("UnityEngine.InputSystem.UI.InputSystemUIInputModule, Unity.InputSystem");
                if (module != null) es.AddComponent(module);
                else es.AddComponent<StandaloneInputModule>();
            }
        }

        private static RectTransform CreateUI(string name, Transform parent)
        {
            var go = new GameObject(name, typeof(RectTransform));
            go.layer = LayerMask.NameToLayer("UI");
            go.transform.SetParent(parent, false);
            return (RectTransform)go.transform;
        }

        private static void Stretch(RectTransform rt, float padding = 0f)
        {
            rt.anchorMin = Vector2.zero;
            rt.anchorMax = Vector2.one;
            rt.offsetMin = new Vector2(padding, padding);
            rt.offsetMax = new Vector2(-padding, -padding);
        }

        private static Font LoadBuiltinFont()
        {
            Font f = null;
            try { f = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf"); } catch { /* eski sürüm */ }
            if (f == null)
            {
                try { f = Resources.GetBuiltinResource<Font>("Arial.ttf"); } catch { /* yeni sürüm */ }
            }
            return f;
        }

        // =====================================================================================  YARDIMCILAR

        private static void SetRef(Object target, string field, Object value)
        {
            var so = new SerializedObject(target);
            SerializedProperty p = so.FindProperty(field);
            if (p == null) { Debug.LogWarning($"[Roguelite] {target.GetType().Name}.{field} bulunamadı"); return; }
            p.objectReferenceValue = value;
            so.ApplyModifiedPropertiesWithoutUndo();
            if (value != null && new SerializedObject(target).FindProperty(field).objectReferenceValue == null)
                Debug.LogError($"[Roguelite] {target.GetType().Name}.{field} atanamadı!", target);
        }

        private static void SetBool(Object target, string field, bool value)
        {
            var so = new SerializedObject(target);
            so.FindProperty(field).boolValue = value;
            so.ApplyModifiedPropertiesWithoutUndo();
        }

        private static void SetInt(Object target, string field, int value)
        {
            var so = new SerializedObject(target);
            so.FindProperty(field).intValue = value;
            so.ApplyModifiedPropertiesWithoutUndo();
        }

        private static void EnsureFolder(string path)
        {
            if (AssetDatabase.IsValidFolder(path)) return;
            string parent = Path.GetDirectoryName(path).Replace('\\', '/');
            EnsureFolder(parent);
            AssetDatabase.CreateFolder(parent, Path.GetFileName(path));
        }

        private static int EnsureLayer(string name)
        {
            int existing = LayerMask.NameToLayer(name);
            if (existing >= 0) return existing;

            var tagManager = new SerializedObject(AssetDatabase.LoadAllAssetsAtPath("ProjectSettings/TagManager.asset")[0]);
            SerializedProperty layers = tagManager.FindProperty("layers");
            for (int i = 8; i < layers.arraySize; i++)
            {
                SerializedProperty l = layers.GetArrayElementAtIndex(i);
                if (string.IsNullOrEmpty(l.stringValue))
                {
                    l.stringValue = name;
                    tagManager.ApplyModifiedPropertiesWithoutUndo();
                    return i;
                }
            }
            throw new InvalidOperationException($"'{name}' katmanı için boş yer yok (Tags and Layers dolu).");
        }

        private static T LoadOrCreateAsset<T>(string path) where T : ScriptableObject
        {
            var asset = AssetDatabase.LoadAssetAtPath<T>(path);
            if (asset != null) return asset;
            asset = ScriptableObject.CreateInstance<T>();
            AssetDatabase.CreateAsset(asset, path);
            return asset;
        }

        private static void AddSceneToBuild(string path)
        {
            var scenes = EditorBuildSettings.scenes;
            foreach (var s in scenes)
                if (s.path == path) return;
            var list = new EditorBuildSettingsScene[scenes.Length + 1];
            list[0] = new EditorBuildSettingsScene(path, true);
            Array.Copy(scenes, 0, list, 1, scenes.Length);
            EditorBuildSettings.scenes = list;
        }

        private static Material FindUnlitSpriteMaterial()
        {
            var mat = AssetDatabase.LoadAssetAtPath<Material>(
                "Packages/com.unity.render-pipelines.universal/Runtime/Materials/Sprite-Unlit-Default.mat");
            if (mat != null) return mat;
            foreach (string guid in AssetDatabase.FindAssets("Sprite-Unlit-Default t:Material"))
            {
                mat = AssetDatabase.LoadAssetAtPath<Material>(AssetDatabase.GUIDToAssetPath(guid));
                if (mat != null) return mat;
            }
            return null; // URP değilse varsayılan Sprites-Default kullanılır
        }

        // =====================================================================================  PROSEDÜREL SPRITE ÇİZİMİ

        private delegate Color PixelPainter(float x, float y, int w, int h);

        private static Sprite CreateSprite(string name, int w, int h, float ppu, Vector2 pivot, PixelPainter painter, bool tiled = false)
        {
            string path = $"{ArtDir}/{name}.png";
            var tex = new Texture2D(w, h, TextureFormat.RGBA32, false);
            var pixels = new Color[w * h];
            for (int y = 0; y < h; y++)
                for (int x = 0; x < w; x++)
                    pixels[y * w + x] = painter(x + 0.5f, y + 0.5f, w, h);
            tex.SetPixels(pixels);
            tex.Apply();
            File.WriteAllBytes(path, tex.EncodeToPNG());
            Object.DestroyImmediate(tex);
            AssetDatabase.ImportAsset(path, ImportAssetOptions.ForceUpdate);

            var ti = (TextureImporter)AssetImporter.GetAtPath(path);
            ti.textureType = TextureImporterType.Sprite;
            ti.spriteImportMode = SpriteImportMode.Single;
            ti.spritePixelsPerUnit = ppu;
            ti.alphaIsTransparency = true;
            ti.mipmapEnabled = false;
            ti.filterMode = FilterMode.Bilinear;
            ti.textureCompression = TextureImporterCompression.Uncompressed;
            ti.wrapMode = tiled ? TextureWrapMode.Repeat : TextureWrapMode.Clamp;

            var settings = new TextureImporterSettings();
            ti.ReadTextureSettings(settings);
            settings.spriteAlignment = (int)SpriteAlignment.Custom;
            settings.spritePivot = pivot;
            settings.spriteMeshType = tiled ? SpriteMeshType.FullRect : SpriteMeshType.Tight;
            ti.SetTextureSettings(settings);
            ti.SaveAndReimport();

            return AssetDatabase.LoadAssetAtPath<Sprite>(path);
        }

        // --- SDF yardımcıları (piksel cinsinden, negatif = içeride)
        private static float Cov(float d) => Mathf.Clamp01(0.5f - d);

        private static float SdEllipse(float x, float y, float cx, float cy, float rx, float ry)
        {
            float px = (x - cx) / rx, py = (y - cy) / ry;
            return (Mathf.Sqrt(px * px + py * py) - 1f) * Mathf.Min(rx, ry);
        }

        private static float SdCircle(float x, float y, float cx, float cy, float r)
            => Mathf.Sqrt((x - cx) * (x - cx) + (y - cy) * (y - cy)) - r;

        private static float SdSegment(float x, float y, float ax, float ay, float bx, float by, float thickness)
        {
            float pax = x - ax, pay = y - ay, bax = bx - ax, bay = by - ay;
            float t = Mathf.Clamp01((pax * bax + pay * bay) / (bax * bax + bay * bay));
            float dx = pax - bax * t, dy = pay - bay * t;
            return Mathf.Sqrt(dx * dx + dy * dy) - thickness * 0.5f;
        }

        private static Color Over(Color dst, Color src, float coverage)
        {
            float a = src.a * coverage;
            if (a <= 0f) return dst;
            float outA = a + dst.a * (1f - a);
            if (outA <= 0f) return Color.clear;
            Color c = (src * a + dst * dst.a * (1f - a)) / outA;
            c.a = outA;
            return c;
        }

        private static Color PaintCharacter(float x, float y, int w, int h)
        {
            Color c = Color.clear;
            Color outline = new Color(0.16f, 0.08f, 0.14f);
            Color body = new Color(1f, 0.62f, 0.38f);
            Color belly = new Color(1f, 0.82f, 0.6f);

            c = Over(c, outline, Cov(SdEllipse(x, y, 64f, 58f, 52f, 56f)));
            c = Over(c, body, Cov(SdEllipse(x, y, 64f, 58f, 47f, 51f)));
            c = Over(c, belly, Cov(SdEllipse(x, y, 64f, 38f, 28f, 22f)) * 0.8f);
            // yanak
            c = Over(c, new Color(1f, 0.4f, 0.45f), Cov(SdEllipse(x, y, 38f, 60f, 9f, 6f)) * 0.6f);
            c = Over(c, new Color(1f, 0.4f, 0.45f), Cov(SdEllipse(x, y, 96f, 60f, 9f, 6f)) * 0.6f);
            // gözler (sağa bakıyor)
            c = Over(c, Color.white, Cov(SdCircle(x, y, 50f, 76f, 12f)));
            c = Over(c, Color.white, Cov(SdCircle(x, y, 82f, 76f, 12f)));
            c = Over(c, outline, Cov(SdCircle(x, y, 54f, 75f, 7f)));
            c = Over(c, outline, Cov(SdCircle(x, y, 86f, 75f, 7f)));
            c = Over(c, Color.white, Cov(SdCircle(x, y, 56f, 78f, 2.5f)));
            c = Over(c, Color.white, Cov(SdCircle(x, y, 88f, 78f, 2.5f)));
            // ağız
            c = Over(c, outline, Cov(SdSegment(x, y, 64f, 60f, 70f, 57f, 3f)));
            c = Over(c, outline, Cov(SdSegment(x, y, 70f, 57f, 76f, 60f, 3f)));
            // parlama
            c = Over(c, new Color(1f, 1f, 1f), Cov(SdEllipse(x, y, 38f, 92f, 8f, 12f)) * 0.45f);
            return c;
        }

        private static Color PaintShadow(float x, float y, int w, int h)
            => Over(Color.clear, new Color(0f, 0f, 0f, 0.35f), Cov(SdEllipse(x, y, w * 0.5f, h * 0.5f, w * 0.46f, h * 0.42f)));

        private static Color PaintCircle(float x, float y, int w, int h)
            => Over(Color.clear, Color.white, Cov(SdCircle(x, y, w * 0.5f, h * 0.5f, w * 0.5f - 2f)));

        private static Color PaintRing(float x, float y, int w, int h)
        {
            float d = Mathf.Abs(SdCircle(x, y, w * 0.5f, h * 0.5f, w * 0.5f - 10f)) - 5f;
            Color c = Over(Color.clear, new Color(1f, 1f, 1f, 0.12f), Cov(SdCircle(x, y, w * 0.5f, h * 0.5f, w * 0.5f - 10f)));
            return Over(c, Color.white, Cov(d));
        }

        private static Color PaintGround(float x, float y, int w, int h)
        {
            Color c = new Color(0.13f, 0.14f, 0.19f, 1f);
            bool edge = x < 1.5f || y < 1.5f;
            if (edge) c = new Color(0.17f, 0.18f, 0.25f, 1f);
            c = Over(c, new Color(0.22f, 0.24f, 0.32f), Cov(SdCircle(x, y, w * 0.5f, h * 0.5f, 2f)));
            return c;
        }

        private static Color PaintDashIcon(float x, float y, int w, int h)
        {
            Color c = Color.clear;
            const float t = 13f;
            c = Over(c, Color.white, Cov(SdSegment(x, y, 34f, 94f, 62f, 64f, t)));
            c = Over(c, Color.white, Cov(SdSegment(x, y, 62f, 64f, 34f, 34f, t)));
            c = Over(c, Color.white, Cov(SdSegment(x, y, 66f, 94f, 94f, 64f, t)));
            c = Over(c, Color.white, Cov(SdSegment(x, y, 94f, 64f, 66f, 34f, t)));
            return c;
        }

        private static Color PaintEnemy(float x, float y, int w, int h)
        {
            Color c = Color.clear;
            Color outline = new Color(0.15f, 0.05f, 0.1f);
            c = Over(c, outline, Cov(SdCircle(x, y, 64f, 64f, 58f)));
            c = Over(c, new Color(0.85f, 0.25f, 0.4f), Cov(SdCircle(x, y, 64f, 64f, 52f)));
            c = Over(c, Color.white, Cov(SdCircle(x, y, 50f, 72f, 10f)));
            c = Over(c, Color.white, Cov(SdCircle(x, y, 78f, 72f, 10f)));
            c = Over(c, outline, Cov(SdCircle(x, y, 50f, 70f, 5f)));
            c = Over(c, outline, Cov(SdCircle(x, y, 78f, 70f, 5f)));
            c = Over(c, outline, Cov(SdSegment(x, y, 40f, 88f, 58f, 80f, 4f))); // kızgın kaşlar
            c = Over(c, outline, Cov(SdSegment(x, y, 88f, 88f, 70f, 80f, 4f)));
            return c;
        }
    }
}
#endif
