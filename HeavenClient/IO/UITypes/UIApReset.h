#pragma once

#include "../UIElement.h"

#include "../../Character/ApResetRules.h"
#include "../../Graphics/Text.h"

#include <array>
#include <cstdint>
#include <functional>

namespace ms
{
	// AP Reset's three dialogs in turn (v83 FUN_00a0a63f type 0x17 on UIWindow.img/Reset/AP, report 195): the
	// stat an AP leaves, the stat it goes to, then the change to confirm. Picking a row moves on at once; only
	// the last has OK. Closing any of them, or Escape, sends nothing.
	class UIApReset : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::APRESET;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `onconfirm` gets the mask bits of the stat the AP goes to and the one it leaves.
		UIApReset(std::function<void(int32_t to, int32_t from)> onconfirm);

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Step
		{
			SOURCE,
			TARGET,
			CONFIRM
		};

		// Each dialog's rows have their own ids, so a click that moves on leaves the next one's button alone.
		enum Buttons : uint16_t
		{
			SOURCE_ROW = 0,
			TARGET_ROW = ap_reset::NUM_ROWS,
			CLOSE = 2 * ap_reset::NUM_ROWS,
			OK,
			CANCEL
		};

		void show(Step step);

		void confirm();

		ap_reset::Stats stats() const;

		std::function<void(int32_t to, int32_t from)> onconfirm;
		Step step;
		ap_reset::Row from;
		ap_reset::Row to;
		std::array<Texture, ap_reset::NUM_ROWS> labels;
		std::array<Texture, ap_reset::NUM_ROWS> grey_labels;
		std::array<Text, ap_reset::NUM_ROWS> values;
		Text less;
		Text less_by;
		Text more;
		Text more_by;
	};
}
