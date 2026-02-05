using Godot;
using System.Collections.Generic;

public static class MeshUtils
{
    public static ArrayMesh BuildMergedMesh(List<MeshPart> parts)
    {
        var positions = new List<Vector3>();
        var normals = new List<Vector3>();
        var uvs = new List<Vector2>();
        var colors = new List<Color>();
        var indices = new List<int>();

        foreach (var part in parts)
        {
            AppendMesh(part.Mesh, part.Transform, positions, normals, uvs, colors, indices, part.ColorOverride);
        }

        var arrays = new Godot.Collections.Array();
        arrays.Resize((int)Mesh.ArrayType.Max);
        arrays[(int)Mesh.ArrayType.Vertex] = positions.ToArray();
        if (normals.Count == positions.Count)
        {
            arrays[(int)Mesh.ArrayType.Normal] = normals.ToArray();
        }
        if (uvs.Count == positions.Count)
        {
            arrays[(int)Mesh.ArrayType.TexUV] = uvs.ToArray();
        }
        if (colors.Count == positions.Count)
        {
            arrays[(int)Mesh.ArrayType.Color] = colors.ToArray();
        }
        arrays[(int)Mesh.ArrayType.Index] = indices.ToArray();

        var mesh = new ArrayMesh();
        mesh.AddSurfaceFromArrays(Mesh.PrimitiveType.Triangles, arrays);
        return mesh;
    }

    private static void AppendMesh(Mesh mesh, Transform3D transform, List<Vector3> positions, List<Vector3> normals, List<Vector2> uvs, List<Color> colors, List<int> indices, Color? overrideColor)
    {
        if (mesh == null || mesh.GetSurfaceCount() == 0)
        {
            return;
        }

        var arrays = mesh.SurfaceGetArrays(0);
        var verts = (Vector3[])arrays[(int)Mesh.ArrayType.Vertex];
        var norms = arrays[(int)Mesh.ArrayType.Normal] as Vector3[];
        var tex = arrays[(int)Mesh.ArrayType.TexUV] as Vector2[];
        var cols = arrays[(int)Mesh.ArrayType.Color] as Color[];
        var idx = (int[])arrays[(int)Mesh.ArrayType.Index];

        var baseIndex = positions.Count;
        var xform = transform;
        var normalXform = transform.Basis.Inverse().Transposed();

        for (int i = 0; i < verts.Length; i++)
        {
            positions.Add(xform * verts[i]);
            if (norms != null)
            {
                normals.Add((normalXform * norms[i]).Normalized());
            }
            if (tex != null)
            {
                uvs.Add(tex[i]);
            }
            if (overrideColor.HasValue)
            {
                colors.Add(overrideColor.Value);
            }
            else if (cols != null)
            {
                colors.Add(cols[i]);
            }
        }

        for (int i = 0; i < idx.Length; i++)
        {
            indices.Add(baseIndex + idx[i]);
        }
    }
}

public class MeshPart
{
    public Mesh Mesh { get; }
    public Transform3D Transform { get; }
    public Color? ColorOverride { get; }

    public MeshPart(Mesh mesh, Transform3D transform, Color? colorOverride = null)
    {
        Mesh = mesh;
        Transform = transform;
        ColorOverride = colorOverride;
    }
}
