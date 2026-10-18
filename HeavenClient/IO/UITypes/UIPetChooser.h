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
	// Which pet a double-clicked piece of pet equipment goes on, when more than one can wear it (v83 CUtilDlgEx
	// mode 6, FUN_009a9b25 on UIWindow.img/UtilDlgEx_Pet). It shows one pet at a time, standing, with its name,
	// level and closeness, and Prev and Next go round them. It is modal: OK or Enter picks the pet shown, and
	// Cancel, the corner button or Escape picks none.
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
		};

		// `onchoose` gets the index in `pets` of the pet picked.
		UIPetChooser(std::vector<Pet> pets, std::function<void(size_t index)> onchoose);

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
		size_t shown;
		Animation standing;
		Texture name_tag;
		Text caption;
		Text name;
		Text level;
		Text closeness;
	};
}
