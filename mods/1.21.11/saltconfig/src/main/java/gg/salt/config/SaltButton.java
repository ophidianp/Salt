package gg.salt.config;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.network.chat.Component;

/**
 * A real Minecraft button (vanilla handles clicks, keyboard focus, Enter and
 * narration) that just draws itself in Salt's flat style instead of stone.
 */
public class SaltButton extends AbstractButton {
	private final Runnable action;
	private final boolean primary;

	public SaltButton(int x, int y, int w, int h, String label, boolean primary, Runnable action) {
		super(x, y, w, h, Component.literal(label));
		this.action = action;
		this.primary = primary;
	}

	@Override
	public void onPress(InputWithModifiers input) {
		action.run();
	}

	@Override
	protected void renderContents(GuiGraphics g, int mouseX, int mouseY, float delta) {
		SaltUi.button(g, Minecraft.getInstance().font, this, getX(), getY(), getWidth(), getHeight(), getMessage().getString(), isHoveredOrFocused(), primary);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		defaultButtonNarrationText(output);
	}
}
