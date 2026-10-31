package gg.salt.menu.mixin.skin;

import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Text boxes: a dark rounded field with an accent outline while typing. */
@Mixin(EditBox.class)
public abstract class EditBoxMixin extends AbstractWidget {
	private EditBoxMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Redirect(method = "extractWidgetRenderState", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphicsExtractor;blitSprite(Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIII)V"))
	private void salt$field(GuiGraphicsExtractor g, RenderPipeline pipeline, Identifier sprite, int x, int y, int w, int h) {
		if (!Skin.on()) {
			g.blitSprite(pipeline, sprite, x, y, w, h);
			return;
		}
		Skin.field(g, x, y, w, h, isFocused(), isHovered(), active);
	}
}
