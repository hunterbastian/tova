from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import KeepInFrame, Paragraph
from reportlab.pdfgen import canvas

OUTPUT_PATH = "output/pdf/tova-app-summary-one-page.pdf"


def build_content(styles):
    content = []

    content.append(Paragraph("<b>Tova App Summary</b>", styles["title"]))
    content.append(Paragraph("Repository-based one-page overview", styles["subtitle"]))

    content.append(Paragraph("<b>What it is</b>", styles["h2"]))
    content.append(
        Paragraph(
            "Tova is a Vite + Three.js single-page first-person exploration app that runs in the browser. "
            "It combines procedural terrain, stylized structures, atmosphere, and movement controls in a real-time render loop.",
            styles["body"],
        )
    )

    content.append(Paragraph("<b>Who it is for</b>", styles["h2"]))
    content.append(
        Paragraph(
            "Primary user/persona: <b>Not found in repo.</b> "
            "Inferred from the implemented controls and UI: desktop keyboard-and-mouse players exploring a fantasy scene.",
            styles["body"],
        )
    )

    content.append(Paragraph("<b>What it does</b>", styles["h2"]))
    feature_bullets = [
        "Pointer-lock first-person movement with WASD/arrow keys and fly/walk chat commands.",
        "Procedural terrain generation with central hill, flattened town zone, ocean drop-off, and valley carving.",
        "Dynamic day/night environment updates for sun, fog, sky color, and light intensities.",
        "World composition modules: ocean tide motion, layered mountain ring, and instanced forest placement.",
        "Procedural structure placement: merged-geometry castle and town aligned to sampled terrain height.",
        "In-game overlays: loading state, crosshair, FPS/coordinates readout, day/night progress bar, and ambient music toggle.",
    ]
    for bullet in feature_bullets:
        content.append(Paragraph(bullet, styles["bullet"]))

    content.append(Paragraph("<b>How it works (repo-evidence architecture)</b>", styles["h2"]))
    arch_bullets = [
        "`index.html` loads `src/main.js`; `main.js` initializes Scene, Camera, WebGLRenderer, and optional postprocessing.",
        "`main.js` creates Environment, Terrain, Ocean, Mountains, Forest, Castle, Town, and Player components.",
        "Player input and chat commands flow to movement logic and environment override (`day`/`night`).",
        "Terrain height queries (`getHeightAt`) drive player ground alignment and structure/vegetation placement.",
        "Animation loop (`THREE.Clock`) updates environment/ocean/player/mountains, then renders via composer or renderer.",
        "External backend services/API calls for gameplay: <b>Not found in repo.</b>",
    ]
    for bullet in arch_bullets:
        content.append(Paragraph(bullet, styles["bullet"]))

    content.append(Paragraph("<b>How to run (minimal)</b>", styles["h2"]))
    run_bullets = [
        "From repo root: `npm install`",
        "Start dev server: `npm run dev`",
        "Open the local Vite URL shown in terminal.",
        "Automated test/lint scripts in `package.json`: <b>Not found in repo.</b>",
    ]
    for bullet in run_bullets:
        content.append(Paragraph(bullet, styles["bullet"]))

    return content


def build_pdf(path):
    c = canvas.Canvas(path, pagesize=letter)
    width, height = letter

    margin_x = 0.6 * inch
    margin_top = 0.6 * inch
    margin_bottom = 0.55 * inch

    styles = getSampleStyleSheet()
    custom = {
        "title": ParagraphStyle(
            "title",
            parent=styles["Normal"],
            fontName="Helvetica-Bold",
            fontSize=15,
            leading=17,
            spaceAfter=3,
            textColor="#111111",
        ),
        "subtitle": ParagraphStyle(
            "subtitle",
            parent=styles["Normal"],
            fontName="Helvetica",
            fontSize=8.4,
            leading=10,
            textColor="#444444",
            spaceAfter=5,
        ),
        "h2": ParagraphStyle(
            "h2",
            parent=styles["Normal"],
            fontName="Helvetica-Bold",
            fontSize=10.2,
            leading=12,
            spaceBefore=4,
            spaceAfter=1,
            textColor="#0f2742",
        ),
        "body": ParagraphStyle(
            "body",
            parent=styles["Normal"],
            fontName="Helvetica",
            fontSize=8.7,
            leading=10.8,
            spaceAfter=2,
            textColor="#111111",
        ),
        "bullet": ParagraphStyle(
            "bullet",
            parent=styles["Normal"],
            fontName="Helvetica",
            fontSize=8.3,
            leading=10.2,
            leftIndent=13,
            firstLineIndent=-7,
            bulletIndent=3,
            bulletFontName="Helvetica",
            bulletFontSize=8.3,
            spaceAfter=1,
            textColor="#111111",
        ),
    }

    content = build_content(custom)

    frame_w = width - (margin_x * 2)
    frame_h = height - margin_top - margin_bottom

    kif = KeepInFrame(frame_w, frame_h, content, mode="shrink")
    w, h = kif.wrapOn(c, frame_w, frame_h)
    kif.drawOn(c, margin_x, height - margin_top - h)

    c.showPage()
    c.save()


if __name__ == "__main__":
    build_pdf(OUTPUT_PATH)
    print(OUTPUT_PATH)
