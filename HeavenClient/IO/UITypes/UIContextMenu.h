#pragma once

#include "../UIElement.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <string>

namespace ms
{
	// v83's menu on someone else (CContextMenu, FUN_00996570), opened by
	// releasing the right button over them: their info, saving their name,
	// trade, whisper, party and guild invites and alerting a GM. It holds
	// the mouse and keys until a row is picked, a press lands outside it or
	// ESC closes it.
	class UIContextMenu : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CONTEXTMENU;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// cursor: where the button came up, in the view.
		UIContextMenu(int32_t cid, const std::string& name, Point<int16_t> cursor);

		void draw(float inter) const override;
		void rightclick(Point<int16_t> cursorpos) override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		int32_t cid;
		std::string name;
		size_t rows;

		Texture top;
		Texture middle;
		Texture bottom;
		Text title;
	};
}
