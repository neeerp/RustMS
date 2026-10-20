#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// A choice of pets (v83 CUtilDlgEx mode 6, FUN_009a9b25 on UIWindow.img/UtilDlgEx_Pet): which pet a
	// double-clicked piece of pet equipment goes on when more than one can wear it, or an NPC's question (NPC
	// dialogue types 9 and 10). It shows one pet at a time, standing, with its name, level and closeness, and
	// Prev and Next go round them. It is modal: OK or Enter picks the pet shown, and Cancel, the corner button
	// or Escape picks none.
	class UIPetChooser : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PETCHOOSER;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		struct Pet
		{
			// Its kind (500xxxx).
			int32_t itemid;
			std::string name;
			int16_t level;
			int16_t closeness;
			// One whose Water of Life has dried up shows its iconD instead.
			bool dead = false;
		};

		// The question for pet equipment (StringPool 0x182).
		static constexpr const char *EQUIP_CAPTION =
				"Please select the pet you would like to put the pet equipment on.";

		// `onchoose` gets the index in `pets` of the pet picked; `oncancel`, if any, is called when none is.
		// `pets` must not be empty.
		UIPetChooser(std::string caption, std::vector<Pet> pets, std::function<void(size_t index)> onchoose,
					 std::function<void()> oncancel = {});

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Show the pet `index` steps round from the first.
		void show(size_t index);

		void choose();

		void cancel();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			EXIT,
			PREV,
			NEXT
		};

		std::vector<Pet> pets;
		std::function<void(size_t index)> onchoose;
		std::function<void()> oncancel;
		size_t shown;
		Animation standing;
		Point<int16_t> standing_at;
		Texture name_tag;
		Text caption;
		Text name;
		Text level;
		Text closeness;
	};
}
