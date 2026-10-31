package gg.salt.menu;

import gg.salt.config.SaltButton;
import gg.salt.config.SaltConfigScreen;
import gg.salt.config.SaltUi;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.client.gui.screens.worldselection.SelectWorldScreen;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;

import java.util.ArrayList;
import java.util.List;

/**
 * Salt's title screen, in the style of Lunar / OneClient: vanilla's
 * panorama behind a centred logo, two big flat buttons, a row of small ones
 * and a bottom bar. Every widget is drawn with {@link SaltUi}, not vanilla's
 * stone buttons, and eases in when the screen opens.
 */
public class SaltTitleScreen extends Screen {
	private static final Identifier LOGO = Identifier.fromNamespaceAndPath("saltmenu", "textures/gui/logo.png");
	private static final String[] SPLASHES = {
		"Lightweight. Clean. Yours.", "Now with 100% more salt.", "Built in Rust, played in Java.",
		"Press Right Shift for saltConfig.", "Sodium + Lithium, seasoned.", "Pass the salt.",
	};

	private record Btn(String id, String label, boolean primary, Runnable action) {}
	private final String splash = SPLASHES[(int) (Math.random() * SPLASHES.length)];

	public SaltTitleScreen() {
		super(Component.literal("Salt"));
	}

	@Override
	protected void init() {
		int cx = width / 2;
		int bw = 200, bh = 22, gap = 4;
		int y = height / 2 - 8;
		place(new Btn("single", "Singleplayer", true, () -> minecraft.gui.setScreen(new SelectWorldScreen(this))), cx - bw / 2, y, bw, bh);
		y += bh + gap;
		place(new Btn("multi", "Multiplayer", false, () -> minecraft.gui.setScreen(new JoinMultiplayerScreen(this))), cx - bw / 2, y, bw, bh);
		y += bh + gap + 2;

		List<Btn> small = new ArrayList<>();
		small.add(new Btn("options", "Options", false, () -> minecraft.gui.setScreen(new OptionsScreen(this, minecraft.options))));
		// saltConfig is the mod list too (it replaces Mod Menu).
		small.add(new Btn("mods", "Mods", false, () -> minecraft.gui.setScreen(new SaltConfigScreen(this))));
		small.add(new Btn("quit", "Quit", false, () -> minecraft.stop()));
		int sw = (bw - gap * (small.size() - 1)) / small.size();
		for (int i = 0; i < small.size(); i++) place(small.get(i), cx - bw / 2 + i * (sw + gap), y, sw, 20);

		if (SaltMenu.CFG.bool("classic")) {
			place(new Btn("classic", "Classic menu", false, () -> {
				SaltMenu.showVanillaOnce = true;
				minecraft.gui.setScreen(new TitleScreen());
			}), width - 86, 6, 80, 16);
		}
	}

	/** Real widgets, so vanilla handles mouse, keyboard focus and Enter. */
	private void place(Btn b, int x, int y, int w, int h) {
		addRenderableWidget(new SaltButton(x, y, w, h, b.label(), b.primary(), b.action()));
	}

	@Override
	public void extractBackground(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
		// Keep vanilla's rotating panorama, then darken it so the UI reads.
		extractPanorama(g, delta);
		g.fillGradient(0, 0, width, height, 0x55000000, 0xC0000000);
	}

	@Override
	public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
		SaltUi.frame();
		super.extractRenderState(g, mouseX, mouseY, delta);
		float open = SaltUi.anim(this, 1, 7);
		int lift = Math.round((1 - open) * 12);

		// logo + wordmark
		int cx = width / 2;
		int top = height / 2 - 96 + lift;
		g.blit(RenderPipelines.GUI_TEXTURED, LOGO, cx - 20, top, 0f, 0f, 40, 40, 40, 40);
		String word = "SALT";
		g.pose().pushMatrix();
		g.pose().translate(cx - font.width(word) * 1.5f, top + 44);
		g.pose().scale(3f, 3f);
		g.text(font, word, 0, 0, SaltUi.alpha(SaltUi.TEXT, open), false);
		g.pose().popMatrix();
		if (SaltMenu.CFG.bool("splash")) {
			g.text(font, splash, cx - font.width(splash) / 2, top + 72, SaltUi.alpha(SaltUi.accent(), open * 0.9f), false);
		}

		// bottom bar
		g.fill(0, height - 20, width, height, 0x66000000);
		g.fill(0, height - 20, width, height - 19, SaltUi.BORDER);
		g.text(font, "Salt · Minecraft " + minecraft.getLaunchedVersion(), 8, height - 14, SaltUi.TEXT_3, false);
		String user = minecraft.getUser().getName();
		int ux = width - font.width(user) - 8;
		SaltUi.box(g, ux - 9, height - 12, 4, 4, 2, 0xFF5EE6A0);
		g.text(font, user, ux, height - 14, SaltUi.TEXT_2, false);
	}

	@Override
	public boolean shouldCloseOnEsc() {
		return false;
	}
}
