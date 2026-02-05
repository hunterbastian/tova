using Godot;

public partial class AudioManager : Node
{
    private AudioStreamPlayer _player;
    private bool _enabled = true;

    public override void _Ready()
    {
        EnsureReverb();
        _player = new AudioStreamPlayer();
        _player.Stream = GD.Load<AudioStream>("res://assets/audio/ambient.mp3");
        _player.Autoplay = true;
        _player.VolumeDb = -14f;
        AddChild(_player);

        _enabled = ConfigLoader.GetBool(ConfigLoader.LoadJson("res://data/ui.json"), "musicEnabled", true);
        SetEnabled(_enabled);
    }

    public void SetEnabled(bool enabled)
    {
        _enabled = enabled;
        if (_enabled)
        {
            if (!_player.Playing) _player.Play();
        }
        else
        {
            _player.Stop();
        }
    }

    private void EnsureReverb()
    {
        var busIndex = AudioServer.GetBusIndex("Master");
        if (busIndex < 0) return;
        if (AudioServer.GetBusEffectCount(busIndex) == 0)
        {
            var reverb = new AudioEffectReverb
            {
                RoomSize = 0.6f,
                Damping = 0.5f,
                Wet = 0.35f,
                Dry = 0.7f
            };
            AudioServer.AddBusEffect(busIndex, reverb, 0);
        }
    }
}
