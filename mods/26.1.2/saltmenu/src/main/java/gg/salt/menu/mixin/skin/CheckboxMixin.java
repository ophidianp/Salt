package gg.salt.menu.mixin.skin;

import com.mojang.blaze3d.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Checkbox;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Checkboxes: a rounded box that fills with the accent and shows a tick. */
@Mixin(Checkbox.class)
public abstract class CheckboxMixin extends AbstractWidget {
	@Shadow
	private boolean selected;

	private CheckboxMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Redirect(method = "extractContents", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphicsExtractor;blitSprite(Lcom/mojang/blaze3d/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIIII)V"))
	private void salt$box(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h, int color) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h, color);
			return;
		}
		Skin.checkbox(g, this, x, y, w, h, selected, isHoveredOrFocused(), alpha);
	}
}
