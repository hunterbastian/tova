using Godot;
using System;

public partial class UIHud : CanvasLayer
{
    public event Action<string> CommandIssued;
    public event Action ChatOpened;
    public event Action ChatClosed;

    private Label _fpsLabel;
    private Label _coordsLabel;
    private Label _timeLabel;
    private TextureRect _timeIcon;
    private ColorRect _timeFill;
    private Button _musicToggle;
    private LineEdit _chatInput;
    private Control _chatContainer;

    private float _fpsTimer = 0f;
    private int _fpsFrames = 0;

    private bool _chatOpen = false;
    private bool _musicEnabled = true;

    private Node3D _player;
    private EnvironmentSystem _env;

    private Texture2D _sunIcon;
    private Texture2D _moonIcon;

    public override void _Ready()
    {
        var config = ConfigLoader.LoadJson("res://data/ui.json");
        _musicEnabled = ConfigLoader.GetBool(config, "musicEnabled", true);

        _player = GetTree().Root.GetNodeOrNull<Node3D>("World/Player");
        _env = GetTree().Root.GetNodeOrNull<EnvironmentSystem>("World/EnvironmentSystem");

        _sunIcon = CreateCircleIcon(new Color("#fff1d0"));
        _moonIcon = CreateMoonIcon(new Color("#a0b6ff"));

        BuildUI();
    }

    private void BuildUI()
    {
        var crosshair = new Control();
        crosshair.AnchorLeft = 0.5f;
        crosshair.AnchorTop = 0.5f;
        crosshair.AnchorRight = 0.5f;
        crosshair.AnchorBottom = 0.5f;
        crosshair.OffsetLeft = -7;
        crosshair.OffsetTop = -7;
        crosshair.OffsetRight = 7;
        crosshair.OffsetBottom = 7;
        AddChild(crosshair);

        var vLine = new ColorRect { Color = new Color(0.94f, 0.95f, 1f, 0.7f) };
        vLine.Size = new Vector2(1, 14);
        vLine.Position = new Vector2(6, 0);
        crosshair.AddChild(vLine);

        var hLine = new ColorRect { Color = new Color(0.94f, 0.95f, 1f, 0.7f) };
        hLine.Size = new Vector2(14, 1);
        hLine.Position = new Vector2(0, 6);
        crosshair.AddChild(hLine);

        var timeBar = new PanelContainer();
        timeBar.Position = new Vector2(0, 16);
        timeBar.AnchorLeft = 0.5f;
        timeBar.AnchorRight = 0.5f;
        timeBar.OffsetLeft = -85;
        timeBar.OffsetRight = 85;
        timeBar.CustomMinimumSize = new Vector2(170, 24);
        timeBar.AddThemeStyleboxOverride("panel", MakePanelStyle(new Color(0.02f, 0.04f, 0.07f, 0.18f), 6));
        AddChild(timeBar);

        var timeVBox = new VBoxContainer();
        timeVBox.AddThemeConstantOverride("separation", 2);
        timeBar.AddChild(timeVBox);

        var timeHBox = new HBoxContainer();
        timeHBox.Alignment = BoxContainer.AlignmentMode.Center;
        timeHBox.AddThemeConstantOverride("separation", 6);
        timeVBox.AddChild(timeHBox);

        _timeIcon = new TextureRect();
        _timeIcon.CustomMinimumSize = new Vector2(12, 12);
        _timeIcon.Texture = _sunIcon;
        timeHBox.AddChild(_timeIcon);

        _timeLabel = new Label();
        _timeLabel.Text = "DAY";
        _timeLabel.AddThemeFontSizeOverride("font_size", 10);
        _timeLabel.AddThemeColorOverride("font_color", new Color("#f0f3ff"));
        timeHBox.AddChild(_timeLabel);

        var barBg = new ColorRect { Color = new Color(1f, 1f, 1f, 0.12f) };
        barBg.CustomMinimumSize = new Vector2(160, 2);
        timeVBox.AddChild(barBg);

        _timeFill = new ColorRect { Color = new Color("#f7b46a") };
        _timeFill.Size = new Vector2(80, 2);
        barBg.AddChild(_timeFill);

        var stats = new PanelContainer();
        stats.Position = new Vector2(16, 16);
        stats.AddThemeStyleboxOverride("panel", MakePanelStyle(new Color(0.02f, 0.04f, 0.07f, 0.2f), 8));
        AddChild(stats);

        var statsBox = new VBoxContainer();
        statsBox.AddThemeConstantOverride("separation", 2);
        stats.AddChild(statsBox);

        _fpsLabel = new Label { Text = "FPS 60" };
        _fpsLabel.AddThemeFontSizeOverride("font_size", 11);
        _fpsLabel.AddThemeColorOverride("font_color", new Color("#dfe7ff"));
        statsBox.AddChild(_fpsLabel);

        _coordsLabel = new Label { Text = "X 0.0 Y 0.0 Z 0.0" };
        _coordsLabel.AddThemeFontSizeOverride("font_size", 11);
        _coordsLabel.AddThemeColorOverride("font_color", new Color("#dfe7ff"));
        statsBox.AddChild(_coordsLabel);

        var hud = new PanelContainer();
        hud.AnchorRight = 1f;
        hud.Position = new Vector2(0, 16);
        hud.OffsetRight = -16;
        hud.OffsetLeft = -170;
        hud.AddThemeStyleboxOverride("panel", MakePanelStyle(new Color(0.02f, 0.04f, 0.07f, 0.16f), 10));
        AddChild(hud);

        var hudBox = new HBoxContainer();
        hudBox.Alignment = BoxContainer.AlignmentMode.Center;
        hudBox.AddThemeConstantOverride("separation", 8);
        hud.AddChild(hudBox);

        hudBox.AddChild(MakeHudItem("WASD"));
        hudBox.AddChild(MakeHudItem("MOUSE"));
        hudBox.AddChild(MakeHudItem("/"));

        _musicToggle = new Button();
        _musicToggle.Text = _musicEnabled ? "Music On" : "Music Off";
        _musicToggle.AnchorRight = 1f;
        _musicToggle.AnchorBottom = 1f;
        _musicToggle.OffsetRight = -16;
        _musicToggle.OffsetBottom = -16;
        _musicToggle.AddThemeStyleboxOverride("normal", MakePanelStyle(new Color(0.02f, 0.04f, 0.07f, 0.45f), 8, new Color(1f, 1f, 1f, 0.2f)));
        _musicToggle.AddThemeStyleboxOverride("hover", MakePanelStyle(new Color(0.04f, 0.06f, 0.12f, 0.5f), 8, new Color(1f, 1f, 1f, 0.35f)));
        _musicToggle.AddThemeColorOverride("font_color", new Color("#f0f3ff"));
        _musicToggle.AddThemeFontSizeOverride("font_size", 11);
        _musicToggle.Pressed += () =>
        {
            _musicEnabled = !_musicEnabled;
            _musicToggle.Text = _musicEnabled ? "Music On" : "Music Off";
            var audio = GetTree().Root.GetNode<AudioManager>("World/AudioManager");
            audio.SetEnabled(_musicEnabled);
        };
        AddChild(_musicToggle);

        _chatContainer = new PanelContainer();
        _chatContainer.AnchorLeft = 0.5f;
        _chatContainer.AnchorRight = 0.5f;
        _chatContainer.AnchorBottom = 1f;
        _chatContainer.OffsetLeft = -160;
        _chatContainer.OffsetRight = 160;
        _chatContainer.OffsetBottom = -20;
        _chatContainer.AddThemeStyleboxOverride("panel", MakePanelStyle(new Color(0f, 0f, 0f, 0.6f), 6));
        _chatContainer.Visible = false;
        AddChild(_chatContainer);

        _chatInput = new LineEdit();
        _chatInput.PlaceholderText = "type command (fly / walk / day / night)";
        _chatInput.TextSubmitted += OnChatSubmitted;
        _chatInput.GuiInput += OnChatInput;
        _chatInput.AddThemeColorOverride("font_color", new Color("#f0f3ff"));
        _chatInput.AddThemeColorOverride("font_placeholder_color", new Color(1f, 1f, 1f, 0.5f));
        _chatInput.AddThemeStyleboxOverride("normal", MakePanelStyle(new Color(0f, 0f, 0f, 0f), 0));
        _chatContainer.AddChild(_chatInput);
    }

    private Control MakeHudItem(string text)
    {
        var panel = new PanelContainer();
        panel.AddThemeStyleboxOverride("panel", MakePanelStyle(new Color(0f, 0f, 0f, 0f), 0));
        var label = new Label { Text = text };
        label.AddThemeFontSizeOverride("font_size", 11);
        label.AddThemeColorOverride("font_color", new Color("#f0f3ff"));
        panel.AddChild(label);
        return panel;
    }

    private StyleBoxFlat MakePanelStyle(Color bg, int radius, Color? border = null)
    {
        var style = new StyleBoxFlat();
        style.BgColor = bg;
        style.CornerRadiusBottomLeft = radius;
        style.CornerRadiusBottomRight = radius;
        style.CornerRadiusTopLeft = radius;
        style.CornerRadiusTopRight = radius;
        if (border.HasValue)
        {
            style.BorderColor = border.Value;
            style.BorderWidthBottom = 1;
            style.BorderWidthTop = 1;
            style.BorderWidthLeft = 1;
            style.BorderWidthRight = 1;
        }
        style.ContentMarginLeft = 6;
        style.ContentMarginRight = 6;
        style.ContentMarginTop = 4;
        style.ContentMarginBottom = 4;
        return style;
    }

    private Texture2D CreateCircleIcon(Color color)
    {
        var image = Image.Create(16, 16, false, Image.Format.Rgba8);
        image.Fill(new Color(0f, 0f, 0f, 0f));
        var center = new Vector2(8, 8);
        for (int y = 0; y < 16; y++)
        {
            for (int x = 0; x < 16; x++)
            {
                var dist = center.DistanceTo(new Vector2(x, y));
                if (dist <= 4.5f)
                {
                    image.SetPixel(x, y, color);
                }
            }
        }
        return ImageTexture.CreateFromImage(image);
    }

    private Texture2D CreateMoonIcon(Color color)
    {
        var image = Image.Create(16, 16, false, Image.Format.Rgba8);
        image.Fill(new Color(0f, 0f, 0f, 0f));
        var center = new Vector2(8, 8);
        var cutCenter = new Vector2(10, 6);
        for (int y = 0; y < 16; y++)
        {
            for (int x = 0; x < 16; x++)
            {
                var dist = center.DistanceTo(new Vector2(x, y));
                var cut = cutCenter.DistanceTo(new Vector2(x, y));
                if (dist <= 5f && cut > 4.5f)
                {
                    image.SetPixel(x, y, color);
                }
            }
        }
        return ImageTexture.CreateFromImage(image);
    }

    private void OnChatSubmitted(string text)
    {
        var cmd = text.Trim().ToLower();
        if (cmd.StartsWith("/")) cmd = cmd.Substring(1);
        if (!string.IsNullOrEmpty(cmd))
        {
            CommandIssued?.Invoke(cmd);
        }
        CloseChat();
    }

    private void OnChatInput(InputEvent @event)
    {
        if (@event is InputEventKey key && key.Pressed && key.Keycode == Key.Escape)
        {
            CloseChat();
        }
    }

    public override void _UnhandledInput(InputEvent @event)
    {
        if (@event.IsActionPressed("open_chat") && !_chatOpen)
        {
            OpenChat();
        }
    }

    private void OpenChat()
    {
        _chatOpen = true;
        _chatContainer.Visible = true;
        _chatInput.Text = "";
        _chatInput.GrabFocus();
        ChatOpened?.Invoke();
    }

    private void CloseChat()
    {
        _chatOpen = false;
        _chatContainer.Visible = false;
        ChatClosed?.Invoke();
    }

    public override void _Process(double delta)
    {
        _fpsFrames++;
        _fpsTimer += (float)delta;
        if (_fpsTimer >= 0.5f)
        {
            var fps = Mathf.RoundToInt(_fpsFrames / _fpsTimer);
            _fpsLabel.Text = $"FPS {fps}";
            _fpsFrames = 0;
            _fpsTimer = 0f;
        }

        if (_player != null)
        {
            var pos = _player.GlobalPosition;
            _coordsLabel.Text = $"X {pos.X:0.0} Y {pos.Y:0.0} Z {pos.Z:0.0}";
        }
    }

    public void UpdateCycle(EnvironmentCycle cycle)
    {
        if (_env == null || cycle.CycleTime <= 0f) return;
        var cycleLength = cycle.IsDay ? _env.DayLength : _env.NightLength;
        var progress = cycleLength == 0f ? 0f : (cycleLength - cycle.TimeToNext) / cycleLength;
        var width = Mathf.Clamp(progress, 0f, 1f) * 160f;
        _timeFill.Size = new Vector2(width, 2f);
        _timeFill.Color = cycle.IsDay ? new Color("#f7b46a") : new Color("#2a3f6e");
        _timeLabel.Text = cycle.IsDay ? "DAY" : "NIGHT";
        _timeIcon.Texture = cycle.IsDay ? _sunIcon : _moonIcon;
    }
}
