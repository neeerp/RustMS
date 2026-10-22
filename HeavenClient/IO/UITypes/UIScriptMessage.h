#pragma once

#include "../UIElement.h"
#include "../Components/FloatingLines.h"

#include "../../Graphics/Text.h"

#include <array>
#include <string>

namespace ms
{
	// SCRIPT_PROGRESS_MESSAGE's lines (FUN_00a13f20): gold Arial bold 15 with a black edge, centred across the
	// screen, the first slot 150 px from the top and each later one 20 px higher (ctor FUN_006cd6ba). See
	// FloatingLines for how long they stay. Clicks go through them.
	class UIScriptMessage : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SCRIPTMESSAGE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIScriptMessage();

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		void show(const std::string &text);

	private:
		FloatingLines lines;
		std::array<Text, FloatingLines::SLOTS> texts, edges;
	};
}
