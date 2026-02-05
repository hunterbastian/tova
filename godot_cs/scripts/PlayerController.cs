using Godot;
using System;

public partial class PlayerController : CharacterBody3D
{
    [Export] public NodePath HudPath;
    [Export] public float LookSensitivity = 0.12f;
    [Export] public float WalkSpeed = 20f;
    [Export] public float FlySpeed = 40f;
    [Export] public float EyeHeight = 3f;
    [Export] public float GroundSnapSpeed = 10f;

    private Camera3D _camera;
    private TerrainGenerator _terrain;
    private UIHud _hud;

    private bool _isFlying = false;
    private float _yaw = 0f;
    private float _pitch = 0f;

    public override void _Ready()
    {
        _camera = GetNode<Camera3D>("Camera3D");
        var shape = new CapsuleShape3D { Radius = 0.4f, Height = 1.8f };
        var collider = GetNode<CollisionShape3D>("CollisionShape3D");
        collider.Shape = shape;

        if (HudPath != null)
        {
            _hud = GetNode<UIHud>(HudPath);
            _hud.CommandIssued += OnCommand;
            _hud.ChatOpened += OnChatOpened;
            _hud.ChatClosed += OnChatClosed;
        }

        Input.MouseMode = Input.MouseModeEnum.Captured;
    }

    public void Initialize(TerrainGenerator terrain)
    {
        _terrain = terrain;
        AlignToTerrain(0f);
    }

    public override void _UnhandledInput(InputEvent @event)
    {
        if (@event is InputEventMouseMotion motion && Input.MouseMode == Input.MouseModeEnum.Captured)
        {
            _yaw -= motion.Relative.X * LookSensitivity * 0.01f;
            _pitch -= motion.Relative.Y * LookSensitivity * 0.01f;
            _pitch = Mathf.Clamp(_pitch, Mathf.DegToRad(-85f), Mathf.DegToRad(85f));
            Rotation = new Vector3(0f, _yaw, 0f);
            _camera.Rotation = new Vector3(_pitch, 0f, 0f);
        }
    }

    public override void _Process(double delta)
    {
        if (Input.IsActionJustPressed("fly_toggle"))
        {
            _isFlying = !_isFlying;
            if (!_isFlying)
            {
                AlignToTerrain(0f);
            }
        }
    }

    public override void _PhysicsProcess(double delta)
    {
        var dt = (float)delta;

        var forward = -_camera.GlobalTransform.Basis.Z;
        forward.Y = 0f;
        forward = forward.Normalized();
        var right = _camera.GlobalTransform.Basis.X;
        right.Y = 0f;
        right = right.Normalized();

        var move = Vector3.Zero;
        move += forward * (Input.GetActionStrength("move_forward") - Input.GetActionStrength("move_back"));
        move += right * (Input.GetActionStrength("move_right") - Input.GetActionStrength("move_left"));
        move += forward * (Input.GetActionStrength("move_forward_gp") - Input.GetActionStrength("move_back_gp"));
        move += right * (Input.GetActionStrength("move_right_gp") - Input.GetActionStrength("move_left_gp"));

        if (move.Length() > 1f) move = move.Normalized();
        var speed = _isFlying ? FlySpeed : WalkSpeed;
        GlobalPosition += move * speed * dt;

        if (_isFlying)
        {
            var vertical = (Input.IsActionPressed("jump") ? 1f : 0f) - (Input.IsActionPressed("sprint") ? 1f : 0f);
            GlobalPosition += Vector3.Up * vertical * FlySpeed * dt;
        }
        else
        {
            AlignToTerrain(dt);
        }

        ApplyGamepadLook(dt);
    }

    private void ApplyGamepadLook(float dt)
    {
        var lookX = Input.GetActionStrength("look_x_gp");
        var lookY = Input.GetActionStrength("look_y_gp");
        if (Mathf.Abs(lookX) > 0.01f || Mathf.Abs(lookY) > 0.01f)
        {
            _yaw -= lookX * LookSensitivity * 0.8f * dt * 60f;
            _pitch -= lookY * LookSensitivity * 0.8f * dt * 60f;
            _pitch = Mathf.Clamp(_pitch, Mathf.DegToRad(-85f), Mathf.DegToRad(85f));
            Rotation = new Vector3(0f, _yaw, 0f);
            _camera.Rotation = new Vector3(_pitch, 0f, 0f);
        }
    }

    private void AlignToTerrain(float dt)
    {
        if (_terrain == null) return;
        var pos = GlobalPosition;
        var groundHeight = _terrain.GetHeightAt(pos.X, pos.Z) + EyeHeight;
        if (dt <= 0f)
        {
            pos.Y = groundHeight;
        }
        else
        {
            var snap = Mathf.Min(1f, GroundSnapSpeed * dt);
            pos.Y = Mathf.Lerp(pos.Y, groundHeight, snap);
        }
        GlobalPosition = pos;
    }

    private void OnCommand(string command)
    {
        if (command == "fly")
        {
            _isFlying = true;
        }
        else if (command == "walk")
        {
            _isFlying = false;
            AlignToTerrain(0f);
        }
    }

    private void OnChatOpened()
    {
        Input.MouseMode = Input.MouseModeEnum.Visible;
    }

    private void OnChatClosed()
    {
        Input.MouseMode = Input.MouseModeEnum.Captured;
    }
}
