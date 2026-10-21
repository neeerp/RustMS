#pragma once

#include "../UIElement.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/Animation.h"

#include <array>

namespace ms
{
	// Vega's Spell's window (v83 FUN_0082b89c on UIWindow.img/VegaSpell): an equip and a scroll dragged in, and Start,
	// which sends them. The server's first 358 starts the roll and says how it went, the second lets it end; once both
	// are in and the gauge is full, the window gives way to the result's (FUN_0082c28b).
	class UIVegaSpell : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::VEGASPELL;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the spell `item_id` in Cash slot `slot`.
		UIVegaSpell(int16_t slot, int32_t item_id);

		~UIVegaSpell() override;

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// 358's code (FUN_0082d8d5).
		void receive(int8_t code);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void start();

		// Shows the result once the gauge is full and both codes are in.
		void finish();

		void close();

		void clear_tooltip();

		// The scroll's rate from Etc's list, 0 if it isn't on it.
		static int32_t listed_rate(int32_t scroll_id);

		void draw_rate(int32_t rate, Point<int16_t> centre) const;

		enum Buttons : uint16_t
		{
			START,
			CLOSE
		};

		enum class State
		{
			IDLE,
			SENT,
			ROLLING,
			DONE
		};

		int16_t slot;
		int32_t item_id;
		// Their slots in the Equip and Use tabs, 0 while empty.
		int16_t equip_slot;
		int16_t scroll_slot;
		int32_t rate;
		State state;
		// Since the roll began, in ms.
		int32_t elapsed;
		// The two codes, '?' until they come.
		int8_t start_code;
		int8_t result_code;
		Texture gauge;
		Texture arrow;
		std::array<Texture, 10> digits;
		Animation arrow_effect;
		Animation spelling;
		Animation twinkling;
		bool arrow_playing;
		bool spelling_playing;
		Sound twinkle;
	};

	// What came of Vega's Spell (FUN_0082da77): SuccessWnd or FailWnd, with the equip and its effect, until OK.
	class UIVegaResult : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::VEGARESULT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIVegaResult(bool success, int16_t equip_slot);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void close();

		int16_t equip_slot;
		Animation effect;
		Point<int16_t> effect_at;
		bool playing;
	};
}
