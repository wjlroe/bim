module TestingThings
  class AThing
    def initialize(something)
      @something = something
    end
  end
end

woot = TestingThings::AThing.new('yup').a_really_long_chained_method_name_that_should_wrap_onto_the_next_line.another_chained_method_call
puts woot.inspect
