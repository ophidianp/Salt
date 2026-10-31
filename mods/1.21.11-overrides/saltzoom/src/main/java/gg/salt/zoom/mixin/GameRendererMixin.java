package gg.salt.zoom.mixin;

import com.llamalad7.mixinextras.injector.ModifyReturnValue;
import gg.salt.zoom.SaltZoom;
import net.minecraft.client.renderer.GameRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/** In 1.21.x the world FOV comes from GameRenderer#getFov(Camera, float, boolean). */
@Mixin(GameRenderer.class)
public class GameRendererMixin {
	@ModifyReturnValue(method = "getFov", at = @At("RETURN"))
	private float saltzoom$zoom(float fov) {
		return SaltZoom.apply(fov);
	}
}
