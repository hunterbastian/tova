using Godot;

public partial class Ocean : Node3D
{
    private MeshInstance3D _mesh;
    private float _baseHeight = -15f;

    public override void _Ready()
    {
        _mesh = new MeshInstance3D();
        var plane = new PlaneMesh
        {
            Size = new Vector2(1000f, 1000f)
        };
        _mesh.Mesh = plane;
        var mat = new StandardMaterial3D
        {
            AlbedoColor = new Color(0.04f, 0.16f, 0.14f),
            Roughness = 0.5f,
            Metallic = 0f
        };
        _mesh.SetSurfaceOverrideMaterial(0, mat);
        _mesh.Rotation = new Vector3(Mathf.DegToRad(-90f), 0f, 0f);
        _mesh.Position = new Vector3(0f, _baseHeight, 0f);
        AddChild(_mesh);
    }

    public override void _Process(double delta)
    {
        var t = (float)Time.GetTicksMsec() / 1000f;
        if (_mesh != null)
        {
            _mesh.Position = new Vector3(0f, _baseHeight + Mathf.Sin(t * 0.5f) * 0.5f, 0f);
        }
    }
}
