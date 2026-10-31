package gg.salt.menu.mixin.skin;

import java.util.function.Function;
import net.minecraft.client.renderer.RenderType;
import gg.salt.menu.skin.Skin;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/** Text boxes: a dark rounded field with an accent outline while typing. */
@Mixin(EditBox.class)
public abstract class EditBoxMixin extends AbstractWidget {
	private EditBoxMixin(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	@Redirect(method = "renderWidget", at = @At(value = "INVOKE",
		target = "Lnet/minecraft/client/gui/GuiGraphics;blitSprite(Lnet/minecraft/resources/ResourceLocation;IIII)V"))
	private void salt$field(GuiGraphics g, ResourceLocation sprite, int x, int y, int w, int h) {
		if (!Skin.on()) {
			g.blitSprite(sprite, x, y, w, h);
			return;
		}
		Skin.field(g, x, y, w, h, isFocused(), isHovered(), active);
	}
}
