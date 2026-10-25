#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <string>
#include <vector>

namespace ms
{
	// The GM's `/block` dialog, Admin/Block (0x997cad, report b176): the
	// days, the reason and a short description, sent by OK as is.
	class UIGmBlock : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GMBLOCK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `reason`: 1 for the first of `reasons`.
		UIGmBlock(const std::string &name, int32_t days, int32_t reason, const std::vector<std::string> &reasons);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Each Etc/BlockReason.img reason's `type`, by its number from 1.
		static std::vector<std::string> reasons();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void confirm();

		// The focus to the other edit.
		void tab();

		void close();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			CLOSE,
			REASON
		};

		std::string name;
		Texture backgrnd;
		Text name_label;
		Textfield days;
		Textfield description;
	};
}
