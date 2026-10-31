package gg.salt.freelook.mixin;

import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.llamalad7.mixinextras.injector.wrapoperation.WrapOperation;
import gg.salt.freelook.SaltFreelook;
import net.minecraft.client.Camera;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/**
 * Camera#alignWithEntity calls setRotation(...) and then moves the camera
 * back for third person. Swapping the rotation there (instead of after)
 * makes the camera orbit around the player correctly.
 */
@Mixin(Camera.class)
public class CameraMixin {
	@WrapOperation(method = "alignWithEntity", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/Camera;setRotation(FF)V"))
	private void saltfreelook$rotation(Camera camera, float yRot, float xRot, Operation<Void> original) {
		if (SaltFreelook.active()) original.call(camera, SaltFreelook.yaw(), SaltFreelook.pitch());
		else original.call(camera, yRot, xRot);
	}
}
