using Godot;
using System.Collections.Generic;

public partial class CastleBuilder : Node3D
{
    private TerrainGenerator _terrain;
    private bool _enableShadows = true;
    private OmniLight3D _glow;

    public void Initialize(TerrainGenerator terrain, bool enableShadows)
    {
        _terrain = terrain;
        _enableShadows = enableShadows;
        Build();
    }

    private void Build()
    {
        var parts = new List<MeshPart>();

        var keep = new BoxMesh { Size = new Vector3(15f, 30f, 15f) };
        var tower = new CylinderMesh { TopRadius = 4f, BottomRadius = 5f, Height = 25f, RadialSegments = 8 };
        var wall = new BoxMesh { Size = new Vector3(30f, 15f, 2f) };

        parts.Add(new MeshPart(keep, new Transform3D(Basis.Identity, new Vector3(0f, 15f, 0f))));

        var towerPositions = new Vector3[]
        {
            new Vector3(15f, 12.5f, 15f),
            new Vector3(-15f, 12.5f, 15f),
            new Vector3(15f, 12.5f, -15f),
            new Vector3(-15f, 12.5f, -15f)
        };
        foreach (var pos in towerPositions)
        {
            parts.Add(new MeshPart(tower, new Transform3D(Basis.Identity, pos)));
        }

        parts.Add(new MeshPart(wall, new Transform3D(Basis.Identity, new Vector3(0f, 7.5f, 15f))));
        parts.Add(new MeshPart(wall, new Transform3D(Basis.Identity, new Vector3(0f, 7.5f, -15f))));
        parts.Add(new MeshPart(wall, new Transform3D(new Basis(new Vector3(0f, 1f, 0f), Mathf.Pi / 2f), new Vector3(15f, 7.5f, 0f))));
        parts.Add(new MeshPart(wall, new Transform3D(new Basis(new Vector3(0f, 1f, 0f), Mathf.Pi / 2f), new Vector3(-15f, 7.5f, 0f))));

        var mesh = MeshUtils.BuildMergedMesh(parts);
        var material = new StandardMaterial3D
        {
            AlbedoColor = new Color(0.53f, 0.53f, 0.53f),
            Roughness = 0.9f,
            Metallic = 0.1f
        };
        mesh.SurfaceSetMaterial(0, material);

        var inst = new MeshInstance3D
        {
            Mesh = mesh,
            CastShadow = _enableShadows,
            ReceiveShadow = _enableShadows
        };

        AddChild(inst);

        var hillHeight = _terrain.GetHeightAt(0f, 0f);
        Position = new Vector3(0f, hillHeight, 0f);

        _glow = new OmniLight3D
        {
            LightColor = new Color("#ffc98b"),
            LightEnergy = 0f,
            OmniRange = 120f,
            ShadowEnabled = false
        };
        _glow.Position = new Vector3(0f, 22f, 0f);
        AddChild(_glow);
    }

    public void SetNightGlow(float intensity)
    {
        if (_glow == null) return;
        _glow.LightEnergy = Mathf.Max(0f, intensity);
    }
}
