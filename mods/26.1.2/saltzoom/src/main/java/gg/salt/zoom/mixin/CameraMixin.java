package gg.salt.zoom.mixin;

import com.llamalad7.mixinextras.injector.ModifyReturnValue;
import gg.salt.zoom.SaltZoom;
import net.minecraft.client.Camera;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/** 26.3 computes the world FOV in Camera#calculateFov (it moved out of GameRenderer). */
@Mixin(Camera.class)
public class CameraMixin {
	@ModifyReturnValue(method = "calculateFov", at = @At("RETURN"))
	private float saltzoom$zoom(float fov) {
		return SaltZoom.apply(fov);
	}
}
