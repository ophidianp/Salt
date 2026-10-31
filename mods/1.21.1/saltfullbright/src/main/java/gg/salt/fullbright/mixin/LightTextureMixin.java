package gg.salt.fullbright.mixin;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import gg.salt.fullbright.SaltFullbright;
import net.minecraft.client.renderer.LightTexture;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * 1.21.x builds the lightmap in LightTexture#updateLightTexture. Fullbright
 * pretends the player has full-strength night vision there (the first
 * hasEffect call is the NIGHT_VISION check), without touching gamma.
 */
@Mixin(LightTexture.class)
public class LightTextureMixin {
	@Shadow
	private boolean updateLightTexture;

	@Inject(method = "updateLightTexture", at = @At("HEAD"))
	private void saltfullbright$refresh(float partialTick, CallbackInfo ci) {
		if (SaltFullbright.consumeDirty()) this.updateLightTexture = true;
	}

	@ModifyExpressionValue(method = "updateLightTexture", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/player/LocalPlayer;hasEffect(Lnet/minecraft/core/Holder;)Z", ordinal = 0))
	private boolean saltfullbright$nightVision(boolean original) {
		return original || SaltFullbright.on();
	}

	@ModifyExpressionValue(method = "updateLightTexture", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/GameRenderer;getNightVisionScale(Lnet/minecraft/world/entity/LivingEntity;F)F"))
	private float saltfullbright$scale(float original) {
		return SaltFullbright.on() ? 1.0f : original;
	}
}
