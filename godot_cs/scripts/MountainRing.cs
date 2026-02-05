using Godot;
using System;
using System.Collections.Generic;

public partial class MountainRing : Node3D
{
    private int _layers = 4;
    private float _radius = 420f;
    private float _height = 210f;
    private int _segments = 200;
    private int _seed = 7381;
    private float _haze = 0.08f;
    private float _baseLift = 12f;
    private float _jaggedness = 0.35f;
    private float _snowlineStart = 0.58f;
    private float _snowlineFade = 0.18f;

    public void Initialize(Godot.Collections.Dictionary config)
    {
        _layers = ConfigLoader.GetInt(config, "mountainLayers", _layers);
        _radius = ConfigLoader.GetFloat(config, "mountainRadius", _radius);
        _height = ConfigLoader.GetFloat(config, "mountainHeight", _height);
        _segments = ConfigLoader.GetInt(config, "mountainSegments", _segments);
        _haze = ConfigLoader.GetFloat(config, "mountainHaze", _haze);
        _baseLift = ConfigLoader.GetFloat(config, "mountainBaseLift", _baseLift);
        _jaggedness = ConfigLoader.GetFloat(config, "mountainJaggedness", _jaggedness);
        _snowlineStart = ConfigLoader.GetFloat(config, "mountainSnowlineStart", _snowlineStart);
        _snowlineFade = ConfigLoader.GetFloat(config, "mountainSnowlineFade", _snowlineFade);

        Build();
    }

    private void Build()
    {
        for (int i = 0; i < _layers; i++)
        {
            var layerSeed = _seed + i * 97;
            var radius = _radius + i * 55f;
            var height = _height + i * 65f;
            var opacity = 1f - i * _haze;

            var mesh = CreateRidgeLayer(radius, height, layerSeed, opacity, i / (float)Mathf.Max(1, _layers - 1));
            var inst = new MeshInstance3D
            {
                Mesh = mesh,
                CastShadow = false,
                ReceiveShadow = false
            };
            AddChild(inst);
        }
    }

    private ArrayMesh CreateRidgeLayer(float radius, float height, int seed, float opacity, float snowBias)
    {
        var rng = new Random(seed);
        var positions = new List<Vector3>();
        var colors = new List<Color>();
        var indices = new List<int>();

        var green = new Color(0.23f, 0.42f, 0.27f);
        var rock = new Color(0.31f, 0.37f, 0.42f);
        var snow = new Color(0.91f, 0.96f, 1.0f);
        var tint = new Color(1f, 1f, 1f).Lerp(new Color(0.76f, 0.84f, 0.94f), snowBias * 0.75f);

        for (int i = 0; i <= _segments; i++)
        {
            var t = i / (float)_segments;
            var angle = t * Mathf.Pi * 2f;
            var noise =
                Mathf.Sin(t * Mathf.Pi * 4f + (float)rng.NextDouble() * 2f) * 0.35f +
                Mathf.Sin(t * Mathf.Pi * 10f + (float)rng.NextDouble() * 4f) * 0.22f +
                Mathf.Sin(t * Mathf.Pi * 22f + (float)rng.NextDouble() * 6f) * (0.12f + _jaggedness * 0.18f) +
                ((float)rng.NextDouble() - 0.5f) * (0.22f + _jaggedness * 0.2f);

            var clamped = Mathf.Clamp(noise, -0.35f, 0.5f);
            var jaggedBoost = 0.08f + _jaggedness * 0.22f;
            var ridgeHeight = height * (0.6f + clamped + Mathf.Max(0f, clamped) * jaggedBoost);
            var ridgeRadius = radius + ((float)rng.NextDouble() - 0.5f) * 12f;

            var x = Mathf.Cos(angle) * ridgeRadius;
            var z = Mathf.Sin(angle) * ridgeRadius;

            var bottom = new Vector3(x, _baseLift, z);
            var top = new Vector3(x, ridgeHeight + _baseLift, z);

            positions.Add(bottom);
            positions.Add(top);

            var heightT = Mathf.Clamp(ridgeHeight / height, 0f, 1f);
            var topColor = green;
            var rockStart = Mathf.Max(0.35f, _snowlineStart - 0.28f);
            var rockFade = 0.32f;
            if (heightT > rockStart)
            {
                topColor = green.Lerp(rock, (heightT - rockStart) / rockFade);
            }
            if (heightT > _snowlineStart)
            {
                topColor = rock.Lerp(snow, (heightT - _snowlineStart) / Mathf.Max(0.08f, _snowlineFade));
            }
            topColor = topColor * tint;
            var bottomColor = green.Lerp(rock, snowBias * 0.25f) * tint;

            colors.Add(bottomColor);
            colors.Add(topColor);
        }

        for (int i = 0; i < _segments; i++)
        {
            var bottomIndex = i * 2;
            var topIndex = i * 2 + 1;
            var bNext = bottomIndex + 2;
            var tNext = topIndex + 2;

            indices.Add(bottomIndex);
            indices.Add(topIndex);
            indices.Add(bNext);

            indices.Add(topIndex);
            indices.Add(tNext);
            indices.Add(bNext);
        }

        var arrays = new Godot.Collections.Array();
        arrays.Resize((int)Mesh.ArrayType.Max);
        arrays[(int)Mesh.ArrayType.Vertex] = positions.ToArray();
        arrays[(int)Mesh.ArrayType.Color] = colors.ToArray();
        arrays[(int)Mesh.ArrayType.Index] = indices.ToArray();

        var mesh = new ArrayMesh();
        mesh.AddSurfaceFromArrays(Mesh.PrimitiveType.Triangles, arrays);

        var mat = new StandardMaterial3D
        {
            VertexColorUseAsAlbedo = true,
            Roughness = 0.95f,
            Metallic = 0f,
            Transparency = opacity < 1f ? BaseMaterial3D.TransparencyEnum.Alpha : BaseMaterial3D.TransparencyEnum.Disabled,
            AlbedoColor = new Color(1f, 1f, 1f, opacity)
        };
        mesh.SurfaceSetMaterial(0, mat);
        return mesh;
    }

    public void UpdateRing(Vector3 playerPosition)
    {
        Position = new Vector3(playerPosition.X, 0f, playerPosition.Z);
    }
}
