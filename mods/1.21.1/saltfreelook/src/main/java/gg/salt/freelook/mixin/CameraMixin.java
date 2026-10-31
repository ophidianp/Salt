package gg.salt.freelook.mixin;

import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.llamalad7.mixinextras.injector.wrapoperation.WrapOperation;
import gg.salt.freelook.SaltFreelook;
import net.minecraft.client.Camera;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/**
 * 1.21.x: Camera#setup calls setRotation(...) and then moves the camera back
 * for third person, so swapping the rotation there orbits correctly.
 */
@Mixin(Camera.class)
public class CameraMixin {
	@WrapOperation(method = "setup", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/Camera;setRotation(FF)V"))
	private void saltfreelook$rotation(Camera camera, float yRot, float xRot, Operation<Void> original) {
		if (SaltFreelook.active()) original.call(camera, SaltFreelook.yaw(), SaltFreelook.pitch());
		else original.call(camera, yRot, xRot);
	}
}
