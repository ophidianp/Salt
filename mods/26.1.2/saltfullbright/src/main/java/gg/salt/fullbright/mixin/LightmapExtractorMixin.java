package gg.salt.fullbright.mixin;

import gg.salt.fullbright.SaltFullbright;
import net.minecraft.client.renderer.LightmapRenderStateExtractor;
import net.minecraft.client.renderer.state.LightmapRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * 26.3 builds the lightmap from a render state. Night-vision intensity 1 +
 * max gamma gives full brightness without touching the gamma option (so the
 * user's real setting is never saved as an out-of-range value).
 */
@Mixin(LightmapRenderStateExtractor.class)
public class LightmapExtractorMixin {
	@Inject(method = "extract", at = @At("TAIL"))
	private void saltfullbright$apply(LightmapRenderState state, float partialTick, CallbackInfo ci) {
		boolean dirty = SaltFullbright.consumeDirty();
		if (SaltFullbright.on()) {
			state.nightVisionEffectIntensity = 1.0f;
			state.brightness = 1.0f;
			state.darknessEffectScale = 0.0f;
		}
		if (dirty) state.needsUpdate = true;
	}
}
