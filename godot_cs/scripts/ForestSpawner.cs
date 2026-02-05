using Godot;
using System;
using System.Collections.Generic;

public partial class ForestSpawner : Node3D
{
    private int _count = 480;
    private float _spruceRatio = 0.75f;
    private Rect2 _region = new Rect2(-60, -230, 290, 460);
    private TerrainGenerator _terrain;
    private bool _enableShadows = true;

    public void Initialize(TerrainGenerator terrain, Godot.Collections.Dictionary config, bool enableShadows)
    {
        _terrain = terrain;
        _count = ConfigLoader.GetInt(config, "forestCount", _count);
        _spruceRatio = ConfigLoader.GetFloat(config, "forestSpruceRatio", _spruceRatio);
        var regionDict = ConfigLoader.GetDict(config, "forestRegion");
        var xMin = ConfigLoader.GetFloat(regionDict, "xMin", _region.Position.X);
        var xMax = ConfigLoader.GetFloat(regionDict, "xMax", _region.Position.X + _region.Size.X);
        var zMin = ConfigLoader.GetFloat(regionDict, "zMin", _region.Position.Y);
        var zMax = ConfigLoader.GetFloat(regionDict, "zMax", _region.Position.Y + _region.Size.Y);
        _region = new Rect2(xMin, zMin, xMax - xMin, zMax - zMin);
        _enableShadows = enableShadows;

        BuildForest();
    }

    private void BuildForest()
    {
        var spruceCount = Mathf.RoundToInt(_count * _spruceRatio);
        var birchCount = _count - spruceCount;

        var spruceTrunk = CreateMultiMesh(new CylinderMesh { TopRadius = 0.45f, BottomRadius = 0.75f, Height = 10f, RadialSegments = 5 },
            new Color(0.23f, 0.17f, 0.12f), spruceCount);
        var spruceLayer = new ConeMesh { BottomRadius = 2.6f, Height = 4.2f, RadialSegments = 6 };
        var spruceLayerA = CreateMultiMesh(spruceLayer, new Color(0.14f, 0.25f, 0.17f), spruceCount);
        var spruceLayerB = CreateMultiMesh(spruceLayer, new Color(0.14f, 0.25f, 0.17f), spruceCount);
        var spruceLayerC = CreateMultiMesh(spruceLayer, new Color(0.14f, 0.25f, 0.17f), spruceCount);

        var birchTrunk = CreateMultiMesh(new CylinderMesh { TopRadius = 0.2f, BottomRadius = 0.35f, Height = 5.5f, RadialSegments = 5 },
            new Color(0.71f, 0.69f, 0.65f), birchCount);
        var birchCanopy = CreateMultiMesh(new SphereMesh { Radius = 1.6f, RadialSegments = 4, Rings = 3 },
            new Color(0.3f, 0.44f, 0.24f), birchCount);

        int sprucePlaced = 0;
        int birchPlaced = 0;
        int attempts = 0;
        int maxAttempts = _count * 8;

        var rng = new Random();
        while ((sprucePlaced < spruceCount || birchPlaced < birchCount) && attempts < maxAttempts)
        {
            attempts++;
            var x = Mathf.Lerp(_region.Position.X, _region.Position.X + _region.Size.X, (float)rng.NextDouble());
            var z = Mathf.Lerp(_region.Position.Y, _region.Position.Y + _region.Size.Y, (float)rng.NextDouble());
            var y = _terrain.GetHeightAt(x, z);
            if (y < -5f) continue;

            var useSpruce = ((float)rng.NextDouble() < _spruceRatio && sprucePlaced < spruceCount) || birchPlaced >= birchCount;
            var rot = (float)rng.NextDouble() * Mathf.Pi * 2f;

            if (useSpruce)
            {
                var trunkHeight = Mathf.Lerp(10f, 16.5f, (float)rng.NextDouble());
                var trunkRadius = Mathf.Lerp(0.45f, 0.8f, (float)rng.NextDouble());
                var layerScale = Mathf.Lerp(0.9f, 1.25f, (float)rng.NextDouble());

                SetInstance(spruceTrunk, sprucePlaced, new Vector3(x, y + (trunkHeight / 2f), z), new Vector3(trunkRadius, trunkHeight / 10f, trunkRadius), rot);
                SetInstance(spruceLayerA, sprucePlaced, new Vector3(x, y + trunkHeight * 0.6f, z), new Vector3(layerScale, layerScale, layerScale), rot);
                SetInstance(spruceLayerB, sprucePlaced, new Vector3(x, y + trunkHeight * 0.78f, z), new Vector3(layerScale * 0.9f, layerScale * 0.9f, layerScale * 0.9f), rot);
                SetInstance(spruceLayerC, sprucePlaced, new Vector3(x, y + trunkHeight * 0.94f, z), new Vector3(layerScale * 0.65f, layerScale * 0.65f, layerScale * 0.65f), rot);
                sprucePlaced++;
            }
            else if (birchPlaced < birchCount)
            {
                var trunkHeight = Mathf.Lerp(4.5f, 7f, (float)rng.NextDouble());
                var trunkRadius = Mathf.Lerp(0.18f, 0.35f, (float)rng.NextDouble());
                var canopyScale = Mathf.Lerp(0.7f, 1.1f, (float)rng.NextDouble());

                SetInstance(birchTrunk, birchPlaced, new Vector3(x, y + (trunkHeight / 2f), z), new Vector3(trunkRadius, trunkHeight / 5.5f, trunkRadius), rot);
                SetInstance(birchCanopy, birchPlaced, new Vector3(x, y + trunkHeight + 0.6f, z), new Vector3(canopyScale, canopyScale * 0.85f, canopyScale), rot);
                birchPlaced++;
            }
        }

        spruceTrunk.Multimesh.VisibleInstanceCount = sprucePlaced;
        spruceLayerA.Multimesh.VisibleInstanceCount = sprucePlaced;
        spruceLayerB.Multimesh.VisibleInstanceCount = sprucePlaced;
        spruceLayerC.Multimesh.VisibleInstanceCount = sprucePlaced;
        birchTrunk.Multimesh.VisibleInstanceCount = birchPlaced;
        birchCanopy.Multimesh.VisibleInstanceCount = birchPlaced;

        AddChild(spruceTrunk);
        AddChild(spruceLayerA);
        AddChild(spruceLayerB);
        AddChild(spruceLayerC);
        AddChild(birchTrunk);
        AddChild(birchCanopy);
    }

    private MultiMeshInstance3D CreateMultiMesh(PrimitiveMesh primitive, Color color, int count)
    {
        var mesh = new ArrayMesh();
        mesh.AddSurfaceFromArrays(Mesh.PrimitiveType.Triangles, primitive.GetMeshArrays());
        var mat = new StandardMaterial3D
        {
            AlbedoColor = color,
            Roughness = 0.9f,
            Metallic = 0f
        };
        mesh.SurfaceSetMaterial(0, mat);

        var mm = new MultiMesh
        {
            Mesh = mesh,
            TransformFormat = MultiMesh.TransformFormatEnum.Transform3D,
            InstanceCount = count,
            VisibleInstanceCount = 0
        };

        var mmi = new MultiMeshInstance3D
        {
            Multimesh = mm,
            CastShadow = _enableShadows,
            ReceiveShadow = _enableShadows
        };

        return mmi;
    }

    private void SetInstance(MultiMeshInstance3D mmi, int idx, Vector3 position, Vector3 scale, float rotY)
    {
        var basis = new Basis(new Vector3(0f, 1f, 0f), rotY).Scaled(scale);
        var xform = new Transform3D(basis, position);
        mmi.Multimesh.SetInstanceTransform(idx, xform);
    }
}
