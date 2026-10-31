package gg.salt.hud.mixin;

import gg.salt.hud.Trackers;
import net.minecraft.client.Minecraft;
import net.minecraft.client.MouseHandler;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Counts mouse presses for the CPS display. Before 1.21.9: MouseHandler#onPress(window, button, action, mods). */
@Mixin(MouseHandler.class)
public class MouseHandlerMixin {
	@Inject(method = "onPress", at = @At("HEAD"))
	private void salthud$countClick(long window, int button, int action, int mods, CallbackInfo ci) {
		if (action == 1 && Minecraft.getInstance().screen == null) {
			Trackers.click(button);
		}
	}
}
