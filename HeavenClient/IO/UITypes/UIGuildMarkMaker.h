#pragma once

#include "../UIElement.h"

#include "../../Character/Guild.h"
#include "../../Character/GuildEmblemRules.h"
#include "../../Character/GuildMark.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <cstdint>

namespace ms
{
	// Lea's emblem maker (v83 FUN_00593c85 on UIWindow.img/UserList/Guild/MakeMark, report 248), which the master's
	// client opens on 65/0x11. The scroll unrolls under the emblem, and a second later the labels, arrows and combo
	// appear: the arrows step through the backgrounds, the marks of the category the combo picks, and their
	// colours, wrapping. Agree, or Enter, asks whether to save it and sends it on yes; no leaves the maker open.
	// Disagree, or Escape, closes it and sends nothing.
	class UIGuildMarkMaker : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GUILDMARKMAKER;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIGuildMarkMaker(const GuildEmblem& current);

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Send the emblem and close.
		void save();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			BACKGROUND_LEFT,
			BACKGROUND_RIGHT,
			BACKGROUND_COLOR_LEFT,
			BACKGROUND_COLOR_RIGHT,
			MARK_LEFT,
			MARK_RIGHT,
			MARK_COLOR_LEFT,
			MARK_COLOR_RIGHT,
			AGREE,
			DISAGREE,
			// Last, so that its list draws over the others.
			CATEGORY
		};

		// FUN_00594533's states.
		enum class Step : uint8_t
		{
			UNROLL,
			WAIT,
			READY
		};

		// The combo, at the window's place, showing the category of the mark.
		void place_category();

		void set_ready();

		void show(const GuildEmblem& emblem);

		void agree();

		Animation backgrnd;
		Texture inner;
		std::array<Texture, 5> messages;
		ColorBox blank;
		GuildMark preview;
		GuildEmblem picked;
		emblem::LastIds last;
		Step step;
		int16_t wait;
	};
}
