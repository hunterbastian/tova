using Godot;
using System;

public partial class TerrainGenerator : Node3D
{
    public float Size { get; private set; } = 500f;
    public int Segments { get; private set; } = 128;
    public float ValleyCenter { get; private set; } = 0f;
    public float ValleyWidth { get; private set; } = 140f;
    public float ValleyDepth { get; private set; } = 18f;
    public string ValleyAxis { get; private set; } = "x";
    public bool EnableShadows { get; private set; } = true;

    private float[,] _heightMap;
    private MeshInstance3D _meshInstance;
    private StaticBody3D _body;

    public override void _Ready()
    {
        var config = ConfigLoader.LoadJson("res://data/terrain.json");
        Size = ConfigLoader.GetFloat(config, "size", Size);
        Segments = ConfigLoader.GetInt(config, "segments", Segments);
        ValleyCenter = ConfigLoader.GetFloat(config, "valleyCenter", ValleyCenter);
        ValleyWidth = ConfigLoader.GetFloat(config, "valleyWidth", ValleyWidth);
        ValleyDepth = ConfigLoader.GetFloat(config, "valleyDepth", ValleyDepth);
        ValleyAxis = ConfigLoader.GetString(config, "valleyAxis", ValleyAxis);
        EnableShadows = ConfigLoader.GetBool(config, "enableShadows", EnableShadows);

        BuildTerrain();
    }

    private float SmoothStep(float edge0, float edge1, float x)
    {
        var t = Mathf.Clamp((x - edge0) / (edge1 - edge0), 0f, 1f);
        return t * t * (3f - 2f * t);
    }

    private void BuildTerrain()
    {
        var vertsPerRow = Segments + 1;
        _heightMap = new float[vertsPerRow, vertsPerRow];

        var positions = new Vector3[vertsPerRow * vertsPerRow];
        var normals = new Vector3[positions.Length];
        var uvs = new Vector2[positions.Length];
        var indices = new int[Segments * Segments * 6];

        var half = Size * 0.5f;
        var step = Size / Segments;

        int v = 0;
        for (int z = 0; z < vertsPerRow; z++)
        {
            for (int x = 0; x < vertsPerRow; x++)
            {
                var worldX = -half + x * step;
                var worldZ = -half + z * step;

                var height = ComputeHeight(worldX, worldZ);
                _heightMap[x, z] = height;

                positions[v] = new Vector3(worldX, height, worldZ);
                uvs[v] = new Vector2((float)x / Segments, (float)z / Segments);
                v++;
            }
        }

        int i = 0;
        for (int z = 0; z < Segments; z++)
        {
            for (int x = 0; x < Segments; x++)
            {
                var i00 = x + vertsPerRow * z;
                var i10 = (x + 1) + vertsPerRow * z;
                var i01 = x + vertsPerRow * (z + 1);
                var i11 = (x + 1) + vertsPerRow * (z + 1);

                indices[i++] = i00;
                indices[i++] = i01;
                indices[i++] = i10;

                indices[i++] = i10;
                indices[i++] = i01;
                indices[i++] = i11;
            }
        }

        for (int t = 0; t < indices.Length; t += 3)
        {
            var i0 = indices[t];
            var i1 = indices[t + 1];
            var i2 = indices[t + 2];
            var p0 = positions[i0];
            var p1 = positions[i1];
            var p2 = positions[i2];
            var normal = (p1 - p0).Cross(p2 - p0).Normalized();
            normals[i0] += normal;
            normals[i1] += normal;
            normals[i2] += normal;
        }
        for (int n = 0; n < normals.Length; n++)
        {
            normals[n] = normals[n].Normalized();
        }

        var arrays = new Godot.Collections.Array();
        arrays.Resize((int)Mesh.ArrayType.Max);
        arrays[(int)Mesh.ArrayType.Vertex] = positions;
        arrays[(int)Mesh.ArrayType.Normal] = normals;
        arrays[(int)Mesh.ArrayType.TexUV] = uvs;
        arrays[(int)Mesh.ArrayType.Index] = indices;

        var mesh = new ArrayMesh();
        mesh.AddSurfaceFromArrays(Mesh.PrimitiveType.Triangles, arrays);
        mesh.SurfaceSetMaterial(0, CreateMaterial());

        _meshInstance = new MeshInstance3D();
        _meshInstance.Mesh = mesh;
        _meshInstance.CastShadow = EnableShadows;
        _meshInstance.ReceiveShadow = EnableShadows;
        AddChild(_meshInstance);

        _body = new StaticBody3D();
        var collider = new CollisionShape3D();
        collider.Shape = mesh.CreateTrimeshShape();
        _body.AddChild(collider);
        AddChild(_body);
    }

    private float ComputeHeight(float x, float z)
    {
        var dist = Mathf.Sqrt(x * x + z * z);
        var height = 0f;

        if (dist < 80f)
        {
            height += 40f * Mathf.Cos((dist / 80f) * (Mathf.Pi / 2f));
        }

        height += Mathf.Sin(x * 0.1f) * Mathf.Sin(z * 0.1f) * 2f;
        height += Mathf.Sin(x * 0.5f) * Mathf.Sin(z * 0.5f) * 0.5f;

        if (x > 20f && x < 100f && z > -50f && z < 50f)
        {
            height *= 0.5f;
        }

        if (x < -100f)
        {
            height -= 20f;
        }

        var axisCoord = ValleyAxis == "z" ? z : x;
        var valleyDistance = Mathf.Abs(axisCoord - ValleyCenter);
        var valleyBlend = 1f - SmoothStep(ValleyWidth * 0.2f, ValleyWidth, valleyDistance);
        if (valleyBlend > 0f)
        {
            var uShape = 1f - Mathf.Pow(valleyDistance / ValleyWidth, 2f);
            height -= ValleyDepth * valleyBlend * Mathf.Max(0f, uShape);
            if (valleyDistance > ValleyWidth * 0.35f)
            {
                height += ValleyDepth * 0.12f * valleyBlend;
            }
            var floorBlend = 1f - SmoothStep(0f, ValleyWidth * 0.25f, valleyDistance);
            height = Mathf.Lerp(height, height * 0.35f, floorBlend * 0.4f);
        }

        height = Mathf.Round(height / 2f) * 2f;
        return height;
    }

    private Material CreateMaterial()
    {
        var mat = new StandardMaterial3D();
        mat.AlbedoColor = new Color(0.23f, 0.37f, 0.04f);
        mat.Roughness = 0.9f;
        mat.Metallic = 0f;
        mat.ShadingMode = BaseMaterial3D.ShadingModeEnum.PerPixel;
        mat.VertexColorUseAsAlbedo = false;
        mat.ParamsDiffuseMode = BaseMaterial3D.DiffuseModeEnum.Lambert;
        return mat;
    }

    public float GetHeightAt(float x, float z)
    {
        var half = Size * 0.5f;
        var localX = x + half;
        var localZ = z + half;

        if (localX < 0 || localZ < 0 || localX > Size || localZ > Size)
        {
            return 0f;
        }

        var step = Size / Segments;
        var gridX = localX / step;
        var gridZ = localZ / step;
        var ix = Mathf.FloorToInt(gridX);
        var iz = Mathf.FloorToInt(gridZ);
        var fx = gridX - ix;
        var fz = gridZ - iz;

        var vertsPerRow = Segments + 1;
        var ix1 = Mathf.Min(ix + 1, Segments);
        var iz1 = Mathf.Min(iz + 1, Segments);

        var z00 = _heightMap[ix, iz];
        var z10 = _heightMap[ix1, iz];
        var z01 = _heightMap[ix, iz1];
        var z11 = _heightMap[ix1, iz1];

        var z0 = Mathf.Lerp(z00, z10, fx);
        var z1 = Mathf.Lerp(z01, z11, fx);
        return Mathf.Lerp(z0, z1, fz);
    }
}
