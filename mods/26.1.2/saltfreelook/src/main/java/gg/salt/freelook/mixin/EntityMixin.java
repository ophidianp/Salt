package gg.salt.freelook.mixin;

import gg.salt.freelook.SaltFreelook;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** While freelooking, mouse movement turns the camera, not the player. */
@Mixin(Entity.class)
public class EntityMixin {
	@Inject(method = "turn", at = @At("HEAD"), cancellable = true)
	private void saltfreelook$turn(double dx, double dy, CallbackInfo ci) {
		if (SaltFreelook.active() && (Object) this == Minecraft.getInstance().player) {
			SaltFreelook.turn(dx, dy);
			ci.cancel();
		}
	}
}
