using Godot;
using System;
using System.Collections.Generic;

public partial class TownBuilder : Node3D
{
    private TerrainGenerator _terrain;
    private bool _enableShadows = true;

    private Dictionary<string, List<MeshPart>> _buckets = new();

    public void Initialize(TerrainGenerator terrain, bool enableShadows)
    {
        _terrain = terrain;
        _enableShadows = enableShadows;
        Build();
    }

    private void AddPart(string key, Mesh mesh, Vector3 position, float rotationY = 0f, Vector3? scale = null)
    {
        var basis = new Basis(new Vector3(0f, 1f, 0f), rotationY);
        if (scale.HasValue)
        {
            basis = basis.Scaled(scale.Value);
        }
        var transform = new Transform3D(basis, position);
        if (!_buckets.ContainsKey(key))
        {
            _buckets[key] = new List<MeshPart>();
        }
        _buckets[key].Add(new MeshPart(mesh, transform));
    }

    private void AddAtTerrain(string key, Mesh mesh, float x, float z, float yOffset = 0f, float rotation = 0f)
    {
        var y = _terrain.GetHeightAt(x, z);
        AddPart(key, mesh, new Vector3(x, y + yOffset, z), rotation);
    }

    private void Build()
    {
        var materials = new Dictionary<string, StandardMaterial3D>
        {
            ["timber"] = MakeMat("#5b3a22"),
            ["plaster"] = MakeMat("#e2d4ba"),
            ["stone"] = MakeMat("#8f8f8f"),
            ["wood"] = MakeMat("#8b5a2b"),
            ["roofRed"] = MakeMat("#7a1f1f"),
            ["roofSlate"] = MakeMat("#39424e"),
            ["road"] = MakeMat("#4b3f34"),
            ["cobble"] = MakeMat("#6f6f6f")
        };

        var baseSmall = new BoxMesh { Size = new Vector3(4f, 3.5f, 4f) };
        var baseWide = new BoxMesh { Size = new Vector3(6f, 4.2f, 5f) };
        var roofPyramid = new ConeMesh { BottomRadius = 3.6f, Height = 2.6f, RadialSegments = 4 };
        var roofLow = new ConeMesh { BottomRadius = 4.1f, Height = 1.8f, RadialSegments = 4 };
        var chimney = new BoxMesh { Size = new Vector3(0.8f, 2.2f, 0.8f) };
        var door = new BoxMesh { Size = new Vector3(0.6f, 1.4f, 0.2f) };
        var window = new BoxMesh { Size = new Vector3(0.5f, 0.5f, 0.15f) };
        var beam = new BoxMesh { Size = new Vector3(0.3f, 3.8f, 0.2f) };
        var roadEW = new BoxMesh { Size = new Vector3(80f, 0.25f, 6f) };
        var roadNS = new BoxMesh { Size = new Vector3(6f, 0.25f, 80f) };
        var plaza = new CylinderMesh { TopRadius = 11f, BottomRadius = 11f, Height = 0.4f, RadialSegments = 20 };
        var wellBase = new CylinderMesh { TopRadius = 2.2f, BottomRadius = 2.4f, Height = 2f, RadialSegments = 12 };
        var wellRoof = new ConeMesh { BottomRadius = 2.6f, Height = 1.6f, RadialSegments = 6 };
        var wellPost = new BoxMesh { Size = new Vector3(0.25f, 2.4f, 0.25f) };
        var stallBase = new BoxMesh { Size = new Vector3(3.6f, 1.2f, 2.2f) };
        var stallRoof = new ConeMesh { BottomRadius = 2.4f, Height = 1.2f, RadialSegments = 4 };
        var tavernSign = new BoxMesh { Size = new Vector3(1.2f, 1.2f, 0.15f) };

        var townCenter = new Vector2(60f, 0f);

        AddAtTerrain("road", roadEW, townCenter.X, townCenter.Y, 0.05f, 0f);
        AddAtTerrain("road", roadNS, townCenter.X - 10f, townCenter.Y, 0.05f, 0f);
        AddAtTerrain("cobble", plaza, townCenter.X, townCenter.Y, 0.1f, 0f);

        AddAtTerrain("stone", wellBase, townCenter.X, townCenter.Y, 1f, 0f);
        AddAtTerrain("roofRed", wellRoof, townCenter.X, townCenter.Y, 2.8f, Mathf.Pi / 6f);
        AddAtTerrain("timber", wellPost, townCenter.X - 1.2f, townCenter.Y + 0.9f, 2.2f, 0f);
        AddAtTerrain("timber", wellPost, townCenter.X + 1.2f, townCenter.Y - 0.9f, 2.2f, 0f);

        for (int i = 0; i < 3; i++)
        {
            var angle = (i / 3f) * Mathf.Pi * 2f + Mathf.Pi / 6f;
            var stallX = townCenter.X + Mathf.Cos(angle) * 13f;
            var stallZ = townCenter.Y + Mathf.Sin(angle) * 13f;
            AddAtTerrain("wood", stallBase, stallX, stallZ, 1f, angle);
            AddAtTerrain("roofRed", stallRoof, stallX, stallZ, 2.2f, angle + Mathf.Pi / 4f);
        }

        var tavernPos = new Vector2(townCenter.X + 16f, townCenter.Y - 10f);
        CreateHouse(baseWide, roofLow, chimney, door, window, beam, tavernPos.X, tavernPos.Y, "stone");
        AddAtTerrain("roofRed", tavernSign, tavernPos.X + 2.6f, tavernPos.Y + 1.2f, 3.2f, Mathf.Pi / 8f);

        var houseRows = new List<Vector2>();
        var rng = new Random();
        for (int i = -3; i <= 3; i++)
        {
            houseRows.Add(new Vector2(townCenter.X + i * 9f + (float)(rng.NextDouble() * 1.5 - 0.75), townCenter.Y - 16f + (float)(rng.NextDouble() * 1.5 - 0.75)));
            houseRows.Add(new Vector2(townCenter.X + i * 9f + (float)(rng.NextDouble() * 1.5 - 0.75), townCenter.Y + 16f + (float)(rng.NextDouble() * 1.5 - 0.75)));
        }
        for (int i = -2; i <= 2; i++)
        {
            houseRows.Add(new Vector2(townCenter.X - 20f + (float)(rng.NextDouble() * 1.5 - 0.75), townCenter.Y + i * 11f + (float)(rng.NextDouble() * 1.5 - 0.75)));
            houseRows.Add(new Vector2(townCenter.X + 26f + (float)(rng.NextDouble() * 1.5 - 0.75), townCenter.Y + i * 11f + (float)(rng.NextDouble() * 1.5 - 0.75)));
        }

        for (int i = 0; i < houseRows.Count; i++)
        {
            var style = (i % 2 == 0) ? "timber" : "stone";
            var pos = houseRows[i];
            CreateHouse(baseWide, roofPyramid, chimney, door, window, beam, pos.X, pos.Y, style);
        }

        foreach (var entry in _buckets)
        {
            var mesh = MeshUtils.BuildMergedMesh(entry.Value);
            mesh.SurfaceSetMaterial(0, materials[entry.Key]);
            var inst = new MeshInstance3D
            {
                Mesh = mesh,
                CastShadow = false,
                ReceiveShadow = _enableShadows
            };
            AddChild(inst);
        }
    }

    private void CreateHouse(BoxMesh baseWide, ConeMesh roofPyramid, BoxMesh chimney, BoxMesh door, BoxMesh window, BoxMesh beam, float x, float z, string style)
    {
        var rotation = (float)(GD.Randf() * Mathf.Pi * 2f);
        var y = _terrain.GetHeightAt(x, z);

        if (style == "stone")
        {
            AddPart("stone", baseWide, new Vector3(x, y + 2.2f, z), rotation);
            AddPart("roofSlate", roofPyramid, new Vector3(x, y + 5.5f, z), rotation + Mathf.Pi / 4f);
        }
        else
        {
            AddPart("plaster", baseWide, new Vector3(x, y + 2.2f, z), rotation);
            AddPart("roofRed", roofPyramid, new Vector3(x, y + 5.5f, z), rotation + Mathf.Pi / 4f);
        }

        AddPart("stone", chimney, new Vector3(x + 1.2f, y + 6.3f, z + 0.8f), rotation * 0.5f);

        if (style == "timber")
        {
            AddPart("timber", beam, new Vector3(x + 1.6f, y + 2.6f, z + 1.9f), rotation);
            AddPart("timber", beam, new Vector3(x - 1.6f, y + 2.6f, z + 1.9f), rotation);
        }

        AddPart("timber", door, new Vector3(x, y + 1.2f, z + 2.05f), rotation);
        AddPart("roofSlate", window, new Vector3(x - 1.1f, y + 2.5f, z + 2.05f), rotation);
        AddPart("roofSlate", window, new Vector3(x + 1.1f, y + 2.5f, z + 2.05f), rotation);
    }

    private StandardMaterial3D MakeMat(string hex)
    {
        return new StandardMaterial3D
        {
            AlbedoColor = new Color(hex),
            Roughness = 1f,
            Metallic = 0f
        };
    }
}
