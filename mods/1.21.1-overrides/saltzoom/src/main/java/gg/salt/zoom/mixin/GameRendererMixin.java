package gg.salt.zoom.mixin;

import com.llamalad7.mixinextras.injector.ModifyReturnValue;
import gg.salt.zoom.SaltZoom;
import net.minecraft.client.renderer.GameRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/** 1.21.1: GameRenderer#getFov(Camera, float, boolean) returns a double. */
@Mixin(GameRenderer.class)
public class GameRendererMixin {
	@ModifyReturnValue(method = "getFov", at = @At("RETURN"))
	private double saltzoom$zoom(double fov) {
		return SaltZoom.apply((float) fov);
	}
}
