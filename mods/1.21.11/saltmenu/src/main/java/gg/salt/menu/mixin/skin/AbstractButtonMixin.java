package gg.salt.menu.mixin.skin;

import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Every vanilla button (Button, CycleButton…) gets Salt's rounded button. */
@Mixin(AbstractButton.class)
public abstract class AbstractButtonMixin extends AbstractWidget {
	private AbstractButtonMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Inject(method = "renderDefaultSprite", at = @At("HEAD"), cancellable = true)
	private void salt$button(GuiGraphics g, CallbackInfo ci) {
		if (!Skin.on()) return;
		Skin.button(g, this, getX(), getY(), getWidth(), getHeight(), active, isHoveredOrFocused(), alpha);
		ci.cancel();
	}
}
