package gg.salt.zoom.mixin;

import gg.salt.zoom.SaltZoom;
import net.minecraft.client.Minecraft;
import net.minecraft.client.MouseHandler;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** While zooming, the scroll wheel changes zoom instead of the hotbar slot. */
@Mixin(MouseHandler.class)
public class MouseHandlerMixin {
	@Inject(method = "onScroll", at = @At("HEAD"), cancellable = true)
	private void saltzoom$scroll(long window, double horizontal, double vertical, CallbackInfo ci) {
		if (Minecraft.getInstance().screen == null && vertical != 0 && SaltZoom.scroll(vertical)) {
			ci.cancel();
		}
	}
}
