#pragma once

#include <array>
#include <cstdint>

#include "../UIDragElement.h"

namespace ms
{
	// The v83 Game Option window, UIWindow.img/GameOpt: whether to allow each
	// kind of request, one check box per row. OK saves the boxes.
	class UIGameOption : public UIDragElement<PosGAMEOPTION>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GAMEOPTION;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIGameOption();

		void draw(float inter) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void save();

		static constexpr size_t ROWS = 12;

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			BOX
		};

		std::array<Configuration::BoolEntry *, ROWS> settings;
		std::array<bool, ROWS> allowed;
		Texture box[2];
	};
}
