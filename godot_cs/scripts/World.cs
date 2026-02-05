using Godot;

public partial class World : Node3D
{
    private TerrainGenerator _terrain;
    private EnvironmentSystem _environment;
    private MountainRing _mountains;
    private ForestSpawner _forest;
    private CastleBuilder _castle;
    private TownBuilder _town;
    private Ocean _ocean;
    private PlayerController _player;
    private UIHud _hud;

    private float _time = 0f;

    public override void _Ready()
    {
        _environment = GetNode<EnvironmentSystem>("EnvironmentSystem");
        _player = GetNode<PlayerController>("Player");
        _hud = GetNode<UIHud>("UIHud");

        _terrain = new TerrainGenerator();
        AddChild(_terrain);

        _ocean = new Ocean();
        AddChild(_ocean);

        _mountains = new MountainRing();
        AddChild(_mountains);

        _forest = new ForestSpawner();
        AddChild(_forest);

        _castle = new CastleBuilder();
        AddChild(_castle);

        _town = new TownBuilder();
        AddChild(_town);

        var worldConfig = ConfigLoader.LoadJson("res://data/worldgen.json");
        _mountains.Initialize(worldConfig);
        _forest.Initialize(_terrain, worldConfig, true);

        var castleEnabled = ConfigLoader.GetBool(worldConfig, "castleEnabled", true);
        var townEnabled = ConfigLoader.GetBool(worldConfig, "townEnabled", true);
        if (castleEnabled)
        {
            _castle.Initialize(_terrain, true);
        }
        if (townEnabled)
        {
            _town.Initialize(_terrain, true);
        }

        _player.GlobalPosition = new Vector3(0f, 20f, 50f);
        _player.HudPath = _hud.GetPath();
        _player.Initialize(_terrain);

        _hud.CommandIssued += OnHudCommand;
    }

    public override void _Process(double delta)
    {
        _time += (float)delta;
        var cycle = _environment.UpdateCycle(_time, _player.GlobalPosition);
        _hud.UpdateCycle(cycle);
        _mountains.UpdateRing(_player.GlobalPosition);
        _castle.SetNightGlow(cycle.Night * 0.55f);
    }

    private void OnHudCommand(string cmd)
    {
        if (cmd == "day")
        {
            _environment.SetOverrideMode("day");
        }
        else if (cmd == "night")
        {
            _environment.SetOverrideMode("night");
        }
    }
}
