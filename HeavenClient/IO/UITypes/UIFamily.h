#pragma once

#include "../UIDragElement.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

namespace ms
{
	// v83's Family window (UIWindow.img/Family, type 0x1b, report b042): the
	// family's name and precepts, the player's juniors and Rep, and the
	// entitlements one at a time. Opening it sends OPEN_FAMILY; the server
	// answers with INFO and the entitlements. OK closes it, and so does Esc
	// when it is the topmost window (b045).
	class UIFamily : public UIDragElement<PosFAMILY>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::FAMILY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIFamily();

		// Opens or closes it, as the Family key does (ToggleUI 0x1b).
		static void toggle();

		void draw(float inter) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// INFO or the entitlements came (0x809820).
		void refresh();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { JUNIOR, TREE, LEFT, RIGHT, PRECEPTS, SPECIAL, OK };

		void show_privilege();
		void enable_buttons();
		void use_privilege();
		void edit_precepts();
		void clear_tooltip();

		Texture background;
		Text title, juniors, reps, today;
		Text name, cost, used, page, description;
		Text precepts;
		// The precepts' left edge, as they scroll across.
		int16_t precepts_x = 0;
		uint16_t precepts_step = 0;
		size_t selected = 0;
		bool tooltip = false;
	};
}
