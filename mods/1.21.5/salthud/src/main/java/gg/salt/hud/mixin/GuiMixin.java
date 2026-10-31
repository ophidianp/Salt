package gg.salt.hud.mixin;

import gg.salt.hud.HudRenderer;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.GuiGraphics;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Before 1.21.6 there is no HUD element registry, so the custom crosshair replaces Gui#renderCrosshair directly. */
@Mixin(Gui.class)
public class GuiMixin {
	@Inject(method = "renderCrosshair", at = @At("HEAD"), cancellable = true)
	private void salthud$crosshair(GuiGraphics g, DeltaTracker delta, CallbackInfo ci) {
		if (HudRenderer.drawCrosshair(g)) ci.cancel();
	}
}
